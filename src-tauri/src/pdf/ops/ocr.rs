//! OCR: turns scanned pages into a searchable PDF.
//!
//! Each page is rendered with Poppler, recognised with Tesseract (which writes
//! a one-page PDF with the page image and an invisible text layer), and the
//! pages are assembled into one document. Everything runs locally with the
//! bundled language data.

use std::path::PathBuf;

use lopdf::Document;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension};
use crate::pdf::assemble::{assemble, PageRef};
use crate::pdf::document::{self, verify_output};
use crate::pdf::ops::{require_one, Staged};
use crate::services::context::{CancelToken, JobContext};
use crate::services::engines::{self, Engine};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OcrOptions {
    /// Tesseract language codes joined with "+", e.g. "eng" or "eng+deu".
    pub language: String,
    /// Rendering resolution; 300 suits most scans.
    pub dpi: u32,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self { language: "eng".into(), dpi: 300 }
    }
}

/// Languages with installed data, excluding the orientation model.
pub fn installed_languages(cancel: &CancelToken) -> AppResult<Vec<String>> {
    let out = engines::run(engines::command(Engine::Tesseract)?, ["--list-langs"], None, cancel, |_| {})?;
    // Tesseract prints the list on stdout (5.x) or stderr (older versions).
    let text = format!("{}\n{}", out.stdout, out.stderr);
    let mut langs: Vec<String> = text
        .lines()
        .skip_while(|l| !l.starts_with("List of available languages"))
        .skip(1)
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && *l != "osd"
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .map(str::to_string)
        .collect();
    langs.sort();
    Ok(langs)
}

pub fn validate_language(spec: &str, installed: &[String]) -> AppResult<String> {
    let parts: Vec<&str> = spec.split('+').map(str::trim).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(AppError::invalid("Choose a language for OCR."));
    }
    for p in &parts {
        if !installed.iter().any(|l| l == p) {
            return Err(AppError::invalid(format!("Language data for “{p}” is not installed."))
                .with_hint("Choose one of the installed languages listed in Settings › OCR."));
        }
    }
    Ok(parts.join("+"))
}

pub fn ocr(ctx: &JobContext, inputs: &[PathBuf], output_name: &str, opts: OcrOptions) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let name = display_name(input);
    let languages = installed_languages(&ctx.cancel)?;
    let language = validate_language(&opts.language, &languages)?;
    let dpi = opts.dpi.clamp(150, 600);
    let doc = document::load(input)?;
    let total = doc.get_pages().len() as u32;
    drop(doc);

    let work = ctx.temp.subdir("ocr")?;
    let source = ctx.stage_input(input)?;
    let mut pages: Vec<Document> = Vec::with_capacity(total as usize);
    for page in 1..=total {
        let base = page as f64 - 1.0;
        let step = |f: f64, msg: String| ctx.progress_file((base + f) / total as f64 * 0.95, msg, &name);

        step(0.0, format!("Rendering page {page} of {total}"));
        let image = work.join(format!("page-{page}"));
        let render_args: Vec<String> = vec![
            "-r".into(),
            dpi.to_string(),
            "-jpeg".into(),
            "-jpegopt".into(),
            "quality=85".into(),
            "-f".into(),
            page.to_string(),
            "-l".into(),
            page.to_string(),
            "-singlefile".into(),
            source.display().to_string(),
            image.display().to_string(),
        ];
        let out = engines::run(engines::command(Engine::Pdftoppm)?, &render_args, None, &ctx.cancel, |_| {})?;
        let jpg = image.with_extension("jpg");
        if !out.success() || !jpg.exists() {
            return Err(AppError::processing(format!("Page {page} could not be rendered."))
                .with_detail(out.stderr_tail()));
        }

        step(0.3, format!("Recognizing text on page {page} of {total}"));
        let out_base = work.join(format!("ocr-{page}"));
        let ocr_args: Vec<String> = vec![
            jpg.display().to_string(),
            out_base.display().to_string(),
            "-l".into(),
            language.clone(),
            "--dpi".into(),
            dpi.to_string(),
            "pdf".into(),
        ];
        let out = engines::run(engines::command(Engine::Tesseract)?, &ocr_args, None, &ctx.cancel, |_| {})?;
        let page_pdf = out_base.with_extension("pdf");
        if !out.success() || !page_pdf.exists() {
            return Err(AppError::processing(format!("Text recognition failed on page {page}."))
                .with_detail(out.stderr_tail()));
        }
        let page_doc = Document::load(&page_pdf).map_err(|e| {
            AppError::processing(format!("Text recognition failed on page {page}."))
                .with_detail(e.to_string())
        })?;
        pages.push(page_doc);
        let _ = std::fs::remove_file(&jpg);
    }

    ctx.check()?;
    ctx.progress(0.96, "Assembling searchable PDF");
    let refs: Vec<PageRef> = (0..pages.len()).map(|i| PageRef::new(i, 1)).collect();
    let mut out = assemble(pages, &refs, false)?;
    let path = ctx.temp.join("ocr.pdf");
    document::save(&mut out, &path)?;
    let mut staged = Staged::single(path.clone(), file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(verify_output(&path, Some(total as usize))?);
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_languages() {
        let installed = vec!["deu".to_string(), "eng".to_string()];
        assert_eq!(validate_language("eng", &installed).unwrap(), "eng");
        assert_eq!(validate_language(" eng + deu ", &installed).unwrap(), "eng+deu");
        assert!(validate_language("fra", &installed).is_err());
        assert!(validate_language("", &installed).is_err());
        assert!(validate_language("eng;rm -rf", &installed).is_err());
    }
}
