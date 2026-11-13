//! Word, PowerPoint, Excel and HTML to PDF with LibreOffice.
//!
//! LibreOffice runs headless with a fresh, private profile for every job.
//! That profile disables macros and blocks linked images from untrusted
//! (remote) locations, so converting a document never runs its code and never
//! reaches the network, even outside the Snap's sandbox.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension};
use crate::pdf::document::verify_output;
use crate::pdf::ops::{require_one, Staged};
use crate::services::context::JobContext;
use crate::services::engines::{self, Engine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeKind {
    Word,
    Presentation,
    Spreadsheet,
    Html,
}

pub const WORD_EXTENSIONS: &[&str] = &["doc", "docx", "docm", "dot", "dotx", "odt", "fodt", "rtf"];
pub const PRESENTATION_EXTENSIONS: &[&str] = &["ppt", "pptx", "pptm", "pps", "ppsx", "odp", "fodp"];
pub const SPREADSHEET_EXTENSIONS: &[&str] = &["xls", "xlsx", "xlsm", "ods", "fods", "csv"];
pub const HTML_EXTENSIONS: &[&str] = &["html", "htm", "xhtml"];

impl OfficeKind {
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            OfficeKind::Word => WORD_EXTENSIONS,
            OfficeKind::Presentation => PRESENTATION_EXTENSIONS,
            OfficeKind::Spreadsheet => SPREADSHEET_EXTENSIONS,
            OfficeKind::Html => HTML_EXTENSIONS,
        }
    }

    /// What the interface calls these files, for error messages.
    pub fn noun(self) -> &'static str {
        match self {
            OfficeKind::Word => "a Word document",
            OfficeKind::Presentation => "a PowerPoint presentation",
            OfficeKind::Spreadsheet => "an Excel spreadsheet",
            OfficeKind::Html => "an HTML page",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            OfficeKind::Word => "Use .docx, .doc, .odt or .rtf files.",
            OfficeKind::Presentation => "Use .pptx, .ppt or .odp files.",
            OfficeKind::Spreadsheet => "Use .xlsx, .xls, .ods or .csv files.",
            OfficeKind::Html => "Use .html or .htm files.",
        }
    }

    /// LibreOffice's PDF export filter for the module that opens this kind.
    fn export_filter(self) -> &'static str {
        match self {
            OfficeKind::Word => "writer_pdf_Export",
            OfficeKind::Presentation => "impress_pdf_Export",
            OfficeKind::Spreadsheet => "calc_pdf_Export",
            OfficeKind::Html => "writer_web_pdf_Export",
        }
    }

    /// The kind a file name belongs to, if any.
    pub fn of(path: &Path) -> Option<OfficeKind> {
        let ext = extension(path);
        [OfficeKind::Word, OfficeKind::Presentation, OfficeKind::Spreadsheet, OfficeKind::Html]
            .into_iter()
            .find(|k| k.extensions().contains(&ext.as_str()))
    }
}

fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Checks an input before it is queued: exists, not empty, right type.
pub fn validate_input(path: &Path, kind: OfficeKind) -> AppResult<u64> {
    let name = display_name(path);
    let meta = fs::metadata(path).map_err(|_| {
        AppError::invalid(format!("“{name}” could not be found."))
            .with_hint("It may have been moved or deleted. Add it again.")
    })?;
    if !meta.is_file() || meta.len() == 0 {
        return Err(AppError::invalid(format!("“{name}” is empty or not a file.")));
    }
    if !kind.extensions().contains(&extension(path).as_str()) {
        return Err(AppError::invalid(format!("“{name}” is not {}.", kind.noun())).with_hint(kind.hint()));
    }
    Ok(meta.len())
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OfficeOptions {
    /// Produce PDF/A-2b, the archival format.
    pub pdf_a: bool,
    /// Spreadsheets: put each sheet on a single page.
    pub one_page_per_sheet: bool,
}

/// The `--convert-to` argument, with export options when any are set.
pub fn convert_to_argument(kind: OfficeKind, opts: &OfficeOptions) -> String {
    let mut options = Vec::new();
    if opts.pdf_a {
        options.push(r#""SelectPdfVersion":{"type":"long","value":"2"}"#);
    }
    if opts.one_page_per_sheet && kind == OfficeKind::Spreadsheet {
        options.push(r#""SinglePageSheets":{"type":"boolean","value":"true"}"#);
    }
    if options.is_empty() {
        format!("pdf:{}", kind.export_filter())
    } else {
        format!("pdf:{}:{{{}}}", kind.export_filter(), options.join(","))
    }
}

/// Profile settings applied to every conversion: no macros, no remote
/// content.
const PROFILE_SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="DisableMacrosExecution" oor:op="fuse"><value>true</value></prop></item>
<item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item>
<item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="BlockUntrustedRefererLinks" oor:op="fuse"><value>true</value></prop></item>
<item oor:path="/org.openoffice.Office.Common/Misc"><prop oor:name="FirstRun" oor:op="fuse"><value>false</value></prop></item>
</oor:items>
"#;

/// A file:// URL for a local folder.
fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => url.push(b as char),
            _ => url.push_str(&format!("%{b:02X}")),
        }
    }
    url
}

pub fn office_to_pdf(
    ctx: &JobContext,
    inputs: &[std::path::PathBuf],
    output_name: &str,
    kind: OfficeKind,
    opts: OfficeOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let name = display_name(input);
    validate_input(input, kind)?;
    ctx.progress_file(0.05, "Preparing", &name);
    let source = ctx.stage_input_as(input, &extension(input))?;

    let profile = ctx.temp.subdir("libreoffice-profile")?;
    fs::create_dir_all(profile.join("user"))?;
    fs::write(profile.join("user/registrymodifications.xcu"), PROFILE_SETTINGS)?;
    let out_dir = ctx.temp.subdir("converted")?;

    // LibreOffice reports no progress, so the bar is indeterminate.
    ctx.status("Converting with LibreOffice");
    let args: Vec<String> = vec![
        "--headless".into(),
        "--invisible".into(),
        "--nologo".into(),
        "--norestore".into(),
        "--nolockcheck".into(),
        "--nodefault".into(),
        format!("-env:UserInstallation={}", file_url(&profile)),
        "--convert-to".into(),
        convert_to_argument(kind, &opts),
        "--outdir".into(),
        out_dir.display().to_string(),
        source.display().to_string(),
    ];
    let out = engines::run(engines::command(Engine::LibreOffice)?, &args, None, &ctx.cancel, |_| {})?;
    let produced = out_dir.join(source.with_extension("pdf").file_name().expect("staged file has a name"));
    if !produced.exists() {
        return Err(AppError::processing(format!("“{name}” could not be converted."))
            .with_hint(
                "The file may be damaged, password-protected, or in a format LibreOffice cannot open. \
             Try opening and re-saving it in its original application.",
            )
            .with_detail(if out.stderr.trim().is_empty() { out.stdout.clone() } else { out.stderr_tail() }));
    }
    ctx.progress(0.95, "Checking the result");
    let pages = verify_output(&produced, None)?;
    let mut staged = Staged::single(produced, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(pages);
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_kinds_by_extension() {
        assert_eq!(OfficeKind::of(Path::new("/a/Report.DOCX")), Some(OfficeKind::Word));
        assert_eq!(OfficeKind::of(Path::new("deck.pptx")), Some(OfficeKind::Presentation));
        assert_eq!(OfficeKind::of(Path::new("budget.csv")), Some(OfficeKind::Spreadsheet));
        assert_eq!(OfficeKind::of(Path::new("page.htm")), Some(OfficeKind::Html));
        assert_eq!(OfficeKind::of(Path::new("photo.png")), None);
        assert_eq!(OfficeKind::of(Path::new("noextension")), None);
    }

    #[test]
    fn validates_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let doc = dir.path().join("a.docx");
        fs::write(&doc, b"PK..").unwrap();
        assert!(validate_input(&doc, OfficeKind::Word).is_ok());
        let err = validate_input(&doc, OfficeKind::Spreadsheet).unwrap_err();
        assert!(err.message.contains("Excel"));
        let empty = dir.path().join("b.docx");
        fs::write(&empty, b"").unwrap();
        assert!(validate_input(&empty, OfficeKind::Word).is_err());
        assert!(validate_input(&dir.path().join("missing.docx"), OfficeKind::Word).is_err());
    }

    #[test]
    fn builds_export_arguments() {
        let none = OfficeOptions::default();
        assert_eq!(convert_to_argument(OfficeKind::Word, &none), "pdf:writer_pdf_Export");
        assert_eq!(convert_to_argument(OfficeKind::Html, &none), "pdf:writer_web_pdf_Export");
        let all = OfficeOptions { pdf_a: true, one_page_per_sheet: true };
        assert_eq!(
            convert_to_argument(OfficeKind::Spreadsheet, &all),
            r#"pdf:calc_pdf_Export:{"SelectPdfVersion":{"type":"long","value":"2"},"SinglePageSheets":{"type":"boolean","value":"true"}}"#
        );
        // Sheet layout only applies to spreadsheets.
        let sheets = OfficeOptions { pdf_a: false, one_page_per_sheet: true };
        assert_eq!(convert_to_argument(OfficeKind::Word, &sheets), "pdf:writer_pdf_Export");
    }

    #[test]
    fn encodes_profile_urls() {
        assert_eq!(file_url(Path::new("/tmp/a b/p")), "file:///tmp/a%20b/p");
    }
}
