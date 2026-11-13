//! Password protection and unlocking with qpdf.
//!
//! Passwords are passed to qpdf through an argument file read from standard
//! input (`@-`), so they never appear in the process list, in logs or on
//! disk. Unlocking requires the document's password: PaperFlow has no way to
//! remove protection without it, by design.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{AppError, AppResult, ErrorKind};
use crate::filesystem::paths::{display_name, file_name_with_extension, validate_pdf_input};
use crate::pdf::ops::{require_one, Staged};
use crate::services::context::JobContext;
use crate::services::engines::{self, Engine};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Encryption {
    #[default]
    Aes256,
    Aes128,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PrintPermission {
    #[default]
    Full,
    /// Low resolution only.
    Low,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProtectOptions {
    pub user_password: String,
    /// Leave empty to generate a random one (restrictions then cannot be lifted).
    pub owner_password: String,
    pub encryption: Encryption,
    pub printing: PrintPermission,
    pub allow_copying: bool,
    pub allow_modifying: bool,
    pub allow_annotating: bool,
    pub allow_form_filling: bool,
}

impl Default for ProtectOptions {
    fn default() -> Self {
        Self {
            user_password: String::new(),
            owner_password: String::new(),
            encryption: Encryption::Aes256,
            printing: PrintPermission::Full,
            allow_copying: true,
            allow_modifying: true,
            allow_annotating: true,
            allow_form_filling: true,
        }
    }
}

fn check_password_text(p: &str) -> AppResult<()> {
    if p.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
        return Err(AppError::invalid("Passwords cannot contain line breaks."));
    }
    Ok(())
}

fn random_password() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

fn yn(b: bool) -> &'static str {
    if b {
        "y"
    } else {
        "n"
    }
}

/// The argument file qpdf reads from stdin.
pub fn protect_argfile(opts: &ProtectOptions, owner: &str, input: &Path, output: &Path) -> String {
    let mut lines = vec![
        "--encrypt".to_string(),
        format!("--user-password={}", opts.user_password),
        format!("--owner-password={owner}"),
    ];
    match opts.encryption {
        Encryption::Aes256 => lines.push("--bits=256".into()),
        Encryption::Aes128 => {
            lines.push("--bits=128".into());
            lines.push("--use-aes=y".into());
        }
    }
    lines.push(format!(
        "--print={}",
        match opts.printing {
            PrintPermission::Full => "full",
            PrintPermission::Low => "low",
            PrintPermission::None => "none",
        }
    ));
    lines.push(format!("--extract={}", yn(opts.allow_copying)));
    lines.push(format!("--modify={}", if opts.allow_modifying { "all" } else if opts.allow_form_filling { "form" } else { "none" }));
    lines.push(format!("--annotate={}", yn(opts.allow_annotating)));
    lines.push(format!("--form={}", yn(opts.allow_form_filling)));
    lines.push("--".into());
    lines.push(input.display().to_string());
    lines.push(output.display().to_string());
    lines.join("\n") + "\n"
}

fn is_encrypted(ctx: &JobContext, input: &Path) -> AppResult<bool> {
    let out = engines::run(engines::command(Engine::Qpdf)?, [&std::ffi::OsString::from("--is-encrypted"), input.as_os_str()], None, &ctx.cancel, |_| {})?;
    match out.status.code() {
        Some(0) => Ok(true),
        Some(2) => Ok(false),
        _ => Err(AppError::invalid(format!("“{}” could not be read.", display_name(input))).with_detail(out.stderr_tail())),
    }
}

/// qpdf exits with 3 when it succeeded with warnings.
fn qpdf_ok(code: Option<i32>) -> bool {
    matches!(code, Some(0) | Some(3))
}

pub fn protect(ctx: &JobContext, inputs: &[PathBuf], output_name: &str, opts: ProtectOptions) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    validate_pdf_input(input)?;
    if opts.user_password.is_empty() {
        return Err(AppError::invalid("Enter a password to open the document."));
    }
    check_password_text(&opts.user_password)?;
    check_password_text(&opts.owner_password)?;
    ctx.progress_file(0.1, "Checking document", display_name(input));
    if is_encrypted(ctx, input)? {
        return Err(AppError::invalid(format!("“{}” is already password-protected.", display_name(input)))
            .with_hint("Unlock it first if you want to change its password."));
    }
    let mut staged = Staged::default();
    let owner = if opts.owner_password.is_empty() { random_password() } else { opts.owner_password.clone() };
    if opts.owner_password.is_empty() {
        staged.warn("No owner password was set, so a random one was used. The restrictions cannot be changed later.");
    }
    let out_path = ctx.temp.join("protected.pdf");
    let argfile = protect_argfile(&opts, &owner, input, &out_path);
    ctx.progress(0.3, "Encrypting");
    let out = engines::run(engines::command(Engine::Qpdf)?, ["@-"], Some(argfile.as_bytes()), &ctx.cancel, |_| {})?;
    if !qpdf_ok(out.status.code()) || !out_path.exists() {
        return Err(AppError::processing("The document could not be encrypted.").with_detail(out.stderr_tail()));
    }
    ctx.progress(0.9, "Checking the result");
    if !is_encrypted(ctx, &out_path)? {
        return Err(AppError::processing("The result failed validation and was discarded."));
    }
    staged.files.push((out_path, file_name_with_extension(output_name, "pdf")));
    Ok(staged)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UnlockOptions {
    pub password: String,
}

pub fn unlock(ctx: &JobContext, inputs: &[PathBuf], output_name: &str, opts: UnlockOptions) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    validate_pdf_input(input)?;
    check_password_text(&opts.password)?;
    ctx.progress_file(0.1, "Checking document", display_name(input));
    if !is_encrypted(ctx, input)? {
        return Err(AppError::invalid(format!("“{}” is not password-protected.", display_name(input))));
    }
    if opts.password.is_empty() {
        return Err(AppError::invalid("Enter the document's password."));
    }
    let out_path = ctx.temp.join("unlocked.pdf");
    let argfile = format!("--password={}\n--decrypt\n--\n{}\n{}\n", opts.password, input.display(), out_path.display());
    ctx.progress(0.3, "Decrypting");
    let out = engines::run(engines::command(Engine::Qpdf)?, ["@-"], Some(argfile.as_bytes()), &ctx.cancel, |_| {})?;
    if !qpdf_ok(out.status.code()) || !out_path.exists() {
        if out.stderr.contains("invalid password") {
            return Err(AppError::new(ErrorKind::WrongPassword, "The password is not correct.")
                .with_hint("Check the password and try again. Passwords are case-sensitive."));
        }
        return Err(AppError::processing("The document could not be unlocked.").with_detail(out.stderr_tail()));
    }
    ctx.progress(0.9, "Checking the result");
    if is_encrypted(ctx, &out_path)? {
        return Err(AppError::processing("The result failed validation and was discarded."));
    }
    let pages = crate::pdf::document::verify_output(&out_path, None)?;
    let mut staged = Staged::single(out_path, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(pages);
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argfile_has_permissions_and_ends_with_files() {
        let opts = ProtectOptions {
            user_password: "u p".into(),
            printing: PrintPermission::None,
            allow_copying: false,
            allow_modifying: false,
            allow_form_filling: false,
            ..Default::default()
        };
        let a = protect_argfile(&opts, "own", Path::new("/a/in.pdf"), Path::new("/b/out.pdf"));
        let lines: Vec<&str> = a.lines().collect();
        assert_eq!(lines[0], "--encrypt");
        assert!(lines.contains(&"--user-password=u p"));
        assert!(lines.contains(&"--owner-password=own"));
        assert!(lines.contains(&"--bits=256"));
        assert!(lines.contains(&"--print=none"));
        assert!(lines.contains(&"--extract=n"));
        assert!(lines.contains(&"--modify=none"));
        assert_eq!(&lines[lines.len() - 3..], &["--", "/a/in.pdf", "/b/out.pdf"]);
    }

    #[test]
    fn rejects_line_breaks_in_passwords() {
        assert!(check_password_text("a\nb").is_err());
        assert!(check_password_text("ok pass").is_ok());
    }

    #[test]
    fn random_passwords_are_long_and_distinct() {
        let a = random_password();
        assert_eq!(a.len(), 64);
        assert_ne!(a, random_password());
    }
}
