//! The unified operation request and result, validation, dispatch, and
//! moving results into place.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{
    create_unique_dir, resolve_output_path, sanitize_file_name, stem, unique_path, validate_image_input,
    validate_output_dir, validate_pdf_input, OverwritePolicy,
};
use crate::pdf::ops::{self, parse_options, Staged};
use crate::services::context::JobContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationType {
    Merge,
    Split,
    Compress,
    Organize,
    Rotate,
    ExtractPages,
    PdfToImages,
    ImagesToPdf,
    ExtractImages,
    PdfToText,
    Watermark,
    PageNumbers,
    Metadata,
    Protect,
    Unlock,
    Ocr,
}

impl OperationType {
    /// Title shown in the job list and recent activity.
    pub fn title(self) -> &'static str {
        use OperationType::*;
        match self {
            Merge => "Merge PDF",
            Split => "Split PDF",
            Compress => "Compress PDF",
            Organize => "Organize PDF",
            Rotate => "Rotate PDF",
            ExtractPages => "Extract Pages",
            PdfToImages => "PDF to Images",
            ImagesToPdf => "Images to PDF",
            ExtractImages => "Extract Images",
            PdfToText => "PDF to Text",
            Watermark => "Watermark PDF",
            PageNumbers => "Page Numbers",
            Metadata => "Edit Metadata",
            Protect => "Protect PDF",
            Unlock => "Unlock PDF",
            Ocr => "OCR PDF",
        }
    }

    /// Suffix for a default output name, e.g. "Report - compressed". Empty
    /// for tools that create a folder of files, which name the folder.
    pub fn default_suffix(self) -> &'static str {
        use OperationType::*;
        match self {
            Merge => "merged",
            Split | PdfToImages | ExtractImages => "",
            Compress => "compressed",
            Organize => "organized",
            Rotate => "rotated",
            ExtractPages => "extract",
            ImagesToPdf => "images",
            PdfToText => "text",
            Watermark => "watermarked",
            PageNumbers => "numbered",
            Metadata => "edited",
            Protect => "protected",
            Unlock => "unlocked",
            Ocr => "searchable",
        }
    }

    pub fn takes_images(self) -> bool {
        self == OperationType::ImagesToPdf
    }
}

/// A request from the interface. Deliberately not `Debug`: options may hold
/// passwords, which must never end up in logs.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfOperation {
    #[serde(rename = "type")]
    pub op_type: OperationType,
    pub input_files: Vec<String>,
    pub output_dir: String,
    #[serde(default)]
    pub output_name: String,
    #[serde(default)]
    pub options: serde_json::Value,
    #[serde(default)]
    pub overwrite: OverwritePolicy,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingResult {
    pub success: bool,
    pub output_files: Vec<String>,
    /// The folder created for multi-file results.
    pub output_folder: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub input_size: u64,
    pub output_size: u64,
    pub page_count: Option<usize>,
}

/// Checks inputs and the output folder before a job is queued, so obvious
/// problems are reported immediately.
pub fn validate(op: &PdfOperation) -> AppResult<Vec<PathBuf>> {
    if op.input_files.is_empty() {
        return Err(AppError::invalid("Add a file first."));
    }
    let mut inputs = Vec::with_capacity(op.input_files.len());
    for f in &op.input_files {
        let p = PathBuf::from(f);
        if !p.is_absolute() {
            return Err(AppError::invalid("Files must be chosen with the file picker or dropped onto the window."));
        }
        if op.op_type.takes_images() {
            validate_image_input(&p)?;
        } else {
            validate_pdf_input(&p)?;
        }
        inputs.push(p);
    }
    let dir = Path::new(&op.output_dir);
    if !dir.is_absolute() {
        return Err(AppError::output("Choose where to save the result."));
    }
    validate_output_dir(dir)?;
    Ok(inputs)
}

/// The output name to use: the user's, or one derived from the first input.
pub fn output_name(op: &PdfOperation, inputs: &[PathBuf]) -> String {
    let given = op.output_name.trim();
    if !given.is_empty() {
        return sanitize_file_name(given);
    }
    let base = inputs.first().map(|p| stem(p)).unwrap_or_else(|| "document".into());
    match op.op_type.default_suffix() {
        "" => sanitize_file_name(&base),
        suffix => sanitize_file_name(&format!("{base} - {suffix}")),
    }
}

/// Runs the operation into the job's temporary folder.
pub fn execute(ctx: &JobContext, op: &PdfOperation, inputs: &[PathBuf]) -> AppResult<Staged> {
    use OperationType::*;
    let name = output_name(op, inputs);
    let o = &op.options;
    match op.op_type {
        Merge => ops::organize::merge(ctx, inputs, &name),
        Split => ops::organize::split(ctx, inputs, &name, parse_options(o)?),
        Organize => ops::organize::organize(ctx, inputs, &name, parse_options(o)?),
        Rotate => ops::organize::rotate(ctx, inputs, &name, parse_options(o)?),
        ExtractPages => ops::organize::extract_pages(ctx, inputs, &name, parse_options(o)?),
        Compress => ops::compress::compress(ctx, inputs, &name, parse_options(o)?),
        PdfToImages => ops::convert::pdf_to_images(ctx, inputs, &name, parse_options(o)?),
        ImagesToPdf => ops::convert::images_to_pdf(ctx, inputs, &name, parse_options(o)?),
        ExtractImages => ops::convert::extract_images(ctx, inputs, &name, parse_options(o)?),
        PdfToText => ops::convert::pdf_to_text(ctx, inputs, &name, parse_options(o)?),
        Watermark => ops::stamp::watermark(ctx, inputs, &name, parse_options(o)?),
        PageNumbers => ops::stamp::page_numbers(ctx, inputs, &name, parse_options(o)?),
        Metadata => ops::metadata::write(ctx, inputs, &name, parse_options(o)?),
        Protect => ops::security::protect(ctx, inputs, &name, parse_options(o)?),
        Unlock => ops::security::unlock(ctx, inputs, &name, parse_options(o)?),
        Ocr => ops::ocr::ocr(ctx, inputs, &name, parse_options(o)?),
    }
}

/// Moves staged results to the output folder and describes the outcome.
pub fn finalize(staged: Staged, op: &PdfOperation, inputs: &[PathBuf]) -> AppResult<ProcessingResult> {
    let dir = PathBuf::from(&op.output_dir);
    validate_output_dir(&dir)?;
    let input_size = inputs.iter().filter_map(|p| fs::metadata(p).ok()).map(|m| m.len()).sum();
    let mut output_files = Vec::with_capacity(staged.files.len());
    let mut output_size = 0;
    let folder = match &staged.folder {
        Some(name) => Some(create_unique_dir(&dir, name)?),
        None => None,
    };
    for (temp, name) in staged.files {
        let dest = match &folder {
            Some(f) => unique_path(f, &sanitize_file_name(&name)),
            None => resolve_output_path(&dir, &sanitize_file_name(&name), op.overwrite, inputs),
        };
        output_size += fs::metadata(&temp).map(|m| m.len()).unwrap_or(0);
        crate::filesystem::paths::move_into_place(&temp, &dest)?;
        output_files.push(dest.display().to_string());
    }
    Ok(ProcessingResult {
        success: true,
        output_files,
        output_folder: folder.map(|f| f.display().to_string()),
        errors: Vec::new(),
        warnings: staged.warnings,
        input_size,
        output_size,
        page_count: staged.page_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::assemble::tests::sample_doc;
    use serde_json::json;

    fn op(value: serde_json::Value) -> PdfOperation {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn parses_requests() {
        let o = op(json!({"type": "pdfToImages", "inputFiles": ["/a.pdf"], "outputDir": "/tmp", "options": {"dpi": 72}}));
        assert_eq!(o.op_type, OperationType::PdfToImages);
        assert_eq!(o.overwrite, OverwritePolicy::Rename);
    }

    #[test]
    fn validates_inputs_and_output() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        sample_doc(1, "A").save(&pdf).unwrap();
        let d = dir.path().display().to_string();
        let ok = op(json!({"type": "compress", "inputFiles": [pdf], "outputDir": d}));
        assert_eq!(validate(&ok).unwrap().len(), 1);
        assert!(validate(&op(json!({"type": "compress", "inputFiles": [], "outputDir": d}))).is_err());
        assert!(validate(&op(json!({"type": "compress", "inputFiles": ["a.pdf"], "outputDir": d}))).is_err());
        assert!(validate(&op(json!({"type": "compress", "inputFiles": [pdf], "outputDir": "relative"}))).is_err());
        assert!(validate(&op(json!({"type": "imagesToPdf", "inputFiles": [pdf], "outputDir": d}))).is_err());
    }

    #[test]
    fn derives_output_names() {
        let o = op(json!({"type": "compress", "inputFiles": ["/x/Report.pdf"], "outputDir": "/tmp"}));
        assert_eq!(output_name(&o, &[PathBuf::from("/x/Report.pdf")]), "Report - compressed");
        let o = op(json!({"type": "compress", "inputFiles": [], "outputDir": "/tmp", "outputName": "../evil"}));
        assert_eq!(output_name(&o, &[]), "-evil");
    }

    #[test]
    fn executes_and_finalizes_without_touching_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        sample_doc(2, "A").save(&pdf).unwrap();
        let original = fs::read(&pdf).unwrap();
        let d = dir.path().display().to_string();
        let o = op(json!({
            "type": "rotate", "inputFiles": [pdf], "outputDir": d,
            "outputName": "a.pdf", "overwrite": "overwrite", "options": {"angle": 90}
        }));
        let inputs = validate(&o).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let staged = execute(&ctx, &o, &inputs).unwrap();
        let result = finalize(staged, &o, &inputs).unwrap();
        assert_eq!(result.output_files, vec![dir.path().join("a (1).pdf").display().to_string()]);
        assert_eq!(fs::read(&pdf).unwrap(), original, "the original is preserved even with overwrite");
        assert_eq!(result.page_count, Some(2));

        let o = op(json!({"type": "split", "inputFiles": [pdf], "outputDir": d, "options": {"mode": "every"}}));
        let staged = execute(&ctx, &o, &inputs).unwrap();
        let result = finalize(staged, &o, &inputs).unwrap();
        assert_eq!(result.output_files.len(), 2);
        assert!(result.output_folder.unwrap().ends_with("a - split"));
    }

    #[test]
    fn bad_options_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        sample_doc(1, "A").save(&pdf).unwrap();
        let o = op(json!({"type": "rotate", "inputFiles": [pdf], "outputDir": "/tmp", "options": {"angle": "sideways"}}));
        let ctx = JobContext::for_tests(dir.path());
        let err = execute(&ctx, &o, &[pdf]).unwrap_err();
        assert!(err.message.contains("settings"));
    }
}
