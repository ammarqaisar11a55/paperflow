//! Path handling: safe file names, unique output paths, input validation and
//! moving finished outputs into place without ever touching the originals.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// How to treat an output path that already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverwritePolicy {
    /// Add " (1)", " (2)", ... to the name. Never replaces anything.
    #[default]
    Rename,
    /// Replace an existing file, except a file that is an input of the job.
    Overwrite,
}

const MAX_NAME_LEN: usize = 180;
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif"];

/// Turns arbitrary text into a file name that is safe on Linux and on the
/// FAT/NTFS drives people plug in: no separators, no control characters, no
/// reserved punctuation, no leading dots, bounded length.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_start_matches('.').trim().trim_end_matches('.').trim();
    let mut out: String = trimmed.chars().take(MAX_NAME_LEN).collect();
    if out.is_empty() {
        out = "document".to_string();
    }
    out
}

/// Sanitizes `name` and makes sure it ends with `.ext` (case-insensitive).
pub fn file_name_with_extension(name: &str, ext: &str) -> String {
    let safe = sanitize_file_name(name);
    let suffix = format!(".{ext}");
    if safe.to_lowercase().ends_with(&suffix.to_lowercase()) && safe.len() > suffix.len() {
        safe
    } else {
        format!("{safe}{suffix}")
    }
}

/// The file name without its extension, e.g. "Report" for "/x/Report.pdf".
pub fn stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "document".into())
}

pub fn display_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

/// Returns `dir/name`, or `dir/name (n).ext` with the smallest free `n`.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (base, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..)
        .map(|n| dir.join(format!("{base} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("an unused file name always exists")
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Chooses the final path for an output file according to the policy. An
/// input of the job is never a valid destination: originals are preserved
/// whatever the policy says.
pub fn resolve_output_path(dir: &Path, name: &str, policy: OverwritePolicy, inputs: &[PathBuf]) -> PathBuf {
    let candidate = dir.join(name);
    let collides_with_input = inputs.iter().any(|i| same_file(i, &candidate));
    match policy {
        OverwritePolicy::Overwrite if !collides_with_input => candidate,
        _ => unique_path(dir, name),
    }
}

/// Creates a new, uniquely named folder inside `dir`.
pub fn create_unique_dir(dir: &Path, name: &str) -> AppResult<PathBuf> {
    let path = unique_path(dir, &sanitize_file_name(name));
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Checks that an output folder exists and is a directory we can write to.
pub fn validate_output_dir(dir: &Path) -> AppResult<()> {
    if dir.as_os_str().is_empty() {
        return Err(AppError::output("Choose where to save the result."));
    }
    if !dir.is_dir() {
        return Err(AppError::output("The save location does not exist.")
            .with_hint("Choose another folder.")
            .with_detail(dir.display().to_string()));
    }
    let meta = fs::metadata(dir)?;
    if meta.permissions().readonly() {
        return Err(AppError::output("The save location is read-only.").with_hint("Choose another folder."));
    }
    Ok(())
}

fn check_regular_file(path: &Path) -> AppResult<fs::Metadata> {
    let meta = fs::metadata(path).map_err(|_| {
        AppError::invalid(format!("“{}” could not be found.", display_name(path)))
            .with_hint("It may have been moved or deleted. Add it again.")
    })?;
    if !meta.is_file() {
        return Err(AppError::invalid(format!("“{}” is not a file.", display_name(path))));
    }
    if meta.len() == 0 {
        return Err(AppError::invalid(format!("“{}” is empty.", display_name(path))));
    }
    Ok(meta)
}

/// Checks the file exists, is not empty, and starts like a PDF.
pub fn validate_pdf_input(path: &Path) -> AppResult<u64> {
    let meta = check_regular_file(path)?;
    let mut head = [0u8; 1024];
    let n = fs::File::open(path)?.read(&mut head)?;
    if !head[..n].windows(5).any(|w| w == b"%PDF-") {
        return Err(AppError::invalid(format!("“{}” is not a PDF file.", display_name(path)))
            .with_hint("Choose a file that ends in .pdf."));
    }
    Ok(meta.len())
}

/// Checks the file exists and is an image type PaperFlow can read.
pub fn validate_image_input(path: &Path) -> AppResult<u64> {
    let meta = check_regular_file(path)?;
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::invalid(format!("“{}” is not a supported image.", display_name(path)))
            .with_hint("Use PNG, JPEG, WebP, BMP, TIFF or GIF images."));
    }
    Ok(meta.len())
}

/// Moves a finished file from the job's temporary folder to its destination.
/// Falls back to copy-and-delete across filesystems.
pub fn move_into_place(from: &Path, to: &Path) -> AppResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Copy to a hidden sibling first so a reader never sees a half-written file.
            let partial = to.with_file_name(format!(
                ".{}.paperflow-partial",
                to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            ));
            fs::copy(from, &partial).map_err(AppError::from)?;
            if let Err(e) = fs::rename(&partial, to) {
                let _ = fs::remove_file(&partial);
                return Err(e.into());
            }
            let _ = fs::remove_file(from);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "-..-etc-passwd");
        assert_eq!(sanitize_file_name("  .hidden  "), "hidden");
        assert_eq!(sanitize_file_name("a\u{0}b\nc"), "a b c");
        assert_eq!(sanitize_file_name("Q1: Report?"), "Q1- Report-");
        assert_eq!(sanitize_file_name(""), "document");
        assert_eq!(sanitize_file_name("..."), "document");
        assert_eq!(sanitize_file_name(&"x".repeat(500)).len(), MAX_NAME_LEN);
    }

    #[test]
    fn adds_extension_once() {
        assert_eq!(file_name_with_extension("Merged", "pdf"), "Merged.pdf");
        assert_eq!(file_name_with_extension("Merged.PDF", "pdf"), "Merged.PDF");
        assert_eq!(file_name_with_extension("notes.txt", "pdf"), "notes.txt.pdf");
        assert_eq!(file_name_with_extension(".pdf", "pdf"), "pdf.pdf");
    }

    #[test]
    fn unique_paths_never_collide() {
        let dir = tempfile::tempdir().unwrap();
        let first = unique_path(dir.path(), "a.pdf");
        assert_eq!(first, dir.path().join("a.pdf"));
        fs::write(&first, b"x").unwrap();
        let second = unique_path(dir.path(), "a.pdf");
        assert_eq!(second, dir.path().join("a (1).pdf"));
        fs::write(&second, b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.pdf"), dir.path().join("a (2).pdf"));
        fs::write(dir.path().join("noext"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "noext"), dir.path().join("noext (1)"));
    }

    #[test]
    fn never_resolves_to_an_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        fs::write(&input, b"%PDF-1.4").unwrap();
        let out = resolve_output_path(
            dir.path(),
            "in.pdf",
            OverwritePolicy::Overwrite,
            std::slice::from_ref(&input),
        );
        assert_ne!(out, input);
        let other = dir.path().join("other.pdf");
        fs::write(&other, b"x").unwrap();
        assert_eq!(
            resolve_output_path(
                dir.path(),
                "other.pdf",
                OverwritePolicy::Overwrite,
                std::slice::from_ref(&input)
            ),
            other
        );
        assert_ne!(resolve_output_path(dir.path(), "other.pdf", OverwritePolicy::Rename, &[input]), other);
    }

    #[test]
    fn validates_pdf_magic() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.pdf");
        fs::write(&good, b"%PDF-1.7\n...").unwrap();
        assert!(validate_pdf_input(&good).is_ok());
        let bad = dir.path().join("bad.pdf");
        fs::write(&bad, b"hello").unwrap();
        assert!(validate_pdf_input(&bad).is_err());
        let empty = dir.path().join("empty.pdf");
        fs::write(&empty, b"").unwrap();
        assert!(validate_pdf_input(&empty).is_err());
        assert!(validate_pdf_input(&dir.path().join("missing.pdf")).is_err());
        assert!(validate_pdf_input(dir.path()).is_err());
    }

    #[test]
    fn validates_image_extension() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("a.PNG");
        fs::write(&png, b"x").unwrap();
        assert!(validate_image_input(&png).is_ok());
        let exe = dir.path().join("a.exe");
        fs::write(&exe, b"x").unwrap();
        assert!(validate_image_input(&exe).is_err());
    }

    #[test]
    fn moves_files() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from.pdf");
        fs::write(&from, b"data").unwrap();
        let to = dir.path().join("sub").join("to.pdf");
        move_into_place(&from, &to).unwrap();
        assert!(!from.exists());
        assert_eq!(fs::read(&to).unwrap(), b"data");
    }

    #[test]
    fn output_dir_must_exist() {
        assert!(validate_output_dir(Path::new("")).is_err());
        assert!(validate_output_dir(Path::new("/definitely/not/here")).is_err());
        let dir = tempfile::tempdir().unwrap();
        assert!(validate_output_dir(dir.path()).is_ok());
    }
}
