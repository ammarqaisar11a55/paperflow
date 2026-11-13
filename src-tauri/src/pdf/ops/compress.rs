//! Compression with Ghostscript's pdfwrite device.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension};
use crate::pdf::document::{self, verify_output};
use crate::pdf::ops::{require_one, Staged};
use crate::services::context::JobContext;
use crate::services::engines::{self, Engine};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    /// Smallest file; images at 72 dpi. For screens and email.
    Extreme,
    /// Good quality, much smaller; images at 150 dpi.
    #[default]
    Recommended,
    /// Print quality; images at 300 dpi.
    Light,
}

impl Preset {
    pub fn pdf_settings(self) -> &'static str {
        match self {
            Preset::Extreme => "/screen",
            Preset::Recommended => "/ebook",
            Preset::Light => "/printer",
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CompressOptions {
    pub preset: Preset,
    /// Overrides the preset's image resolution.
    pub image_dpi: Option<u32>,
    pub grayscale: bool,
}

pub fn ghostscript_args(opts: &CompressOptions, input: &str, output: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "-sDEVICE=pdfwrite",
        "-dCompatibilityLevel=1.6",
        "-dNOPAUSE",
        "-dBATCH",
        "-dSAFER",
        "-dDetectDuplicateImages=true",
        "-dCompressFonts=true",
        "-dSubsetFonts=true",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(format!("-dPDFSETTINGS={}", opts.preset.pdf_settings()));
    if let Some(dpi) = opts.image_dpi {
        let dpi = dpi.clamp(36, 1200);
        for kind in ["Color", "Gray"] {
            args.push(format!("-dDownsample{kind}Images=true"));
            args.push(format!("-d{kind}ImageDownsampleType=/Bicubic"));
            args.push(format!("-d{kind}ImageResolution={dpi}"));
            args.push(format!("-d{kind}ImageDownsampleThreshold=1.0"));
        }
        args.push("-dDownsampleMonoImages=true".into());
        args.push(format!("-dMonoImageResolution={}", (dpi * 2).min(1200)));
    }
    if opts.grayscale {
        args.push("-sColorConversionStrategy=Gray".into());
        args.push("-dProcessColorModel=/DeviceGray".into());
    }
    args.push(format!("-sOutputFile={output}"));
    // Everything after "--" is a file name, never an option.
    args.push("--".into());
    args.push(input.to_string());
    args
}

pub fn compress(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: CompressOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let name = display_name(input);
    let doc = document::load(input)?;
    let total = doc.get_pages().len();
    drop(doc);
    let original_size = fs::metadata(input)?.len();

    let out_path = ctx.temp.join("compressed.pdf");
    ctx.progress_file(0.02, "Optimizing document", &name);
    let staged_input = ctx.stage_input(input)?;
    let args = ghostscript_args(&opts, &staged_input.display().to_string(), &out_path.display().to_string());
    let output = engines::run(engines::command(Engine::Ghostscript)?, &args, None, &ctx.cancel, |line| {
        if let Some(n) = line.strip_prefix("Page ").and_then(|r| r.trim().parse::<usize>().ok()) {
            ctx.progress_file(
                0.02 + 0.9 * n as f64 / total.max(1) as f64,
                format!("Optimizing page {n} of {total}"),
                &name,
            );
        }
    })?;
    if !output.success() || !out_path.exists() {
        return Err(AppError::processing("Ghostscript could not compress this document.")
            .with_hint("The file may be damaged. Try re-saving it from another PDF viewer.")
            .with_detail(output.stderr_tail()));
    }
    ctx.progress(0.95, "Checking the result");
    verify_output(&out_path, Some(total))?;

    let mut staged = Staged { page_count: Some(total), ..Default::default() };
    let new_size = fs::metadata(&out_path)?.len();
    let final_path = if new_size >= original_size {
        staged.warn(
            "This document is already well optimized. The result is an unchanged copy of the original.",
        );
        let copy = ctx.temp.join("original-copy.pdf");
        fs::copy(input, &copy)?;
        copy
    } else {
        out_path
    };
    staged.files.push((final_path, file_name_with_extension(output_name, "pdf")));
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_safe_ghostscript_arguments() {
        let opts = CompressOptions { preset: Preset::Extreme, image_dpi: Some(100), grayscale: true };
        let args = ghostscript_args(&opts, "-in.pdf", "/tmp/out.pdf");
        assert!(args.contains(&"-dSAFER".to_string()));
        assert!(args.contains(&"-dPDFSETTINGS=/screen".to_string()));
        assert!(args.contains(&"-dColorImageResolution=100".to_string()));
        assert!(args.contains(&"-sColorConversionStrategy=Gray".to_string()));
        // The input comes after "--" so a name starting with "-" is not an option.
        let n = args.len();
        assert_eq!(args[n - 2], "--");
        assert_eq!(args[n - 1], "-in.pdf");
    }
}
