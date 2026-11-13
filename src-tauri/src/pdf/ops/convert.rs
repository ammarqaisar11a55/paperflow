//! Conversions: images to PDF (native), and PDF to images, text and
//! extracted images (Poppler).

use std::fs;
use std::path::PathBuf;

use lopdf::{dictionary, Document, Object};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{
    display_name, file_name_with_extension, sanitize_file_name, stem, validate_image_input,
};
use crate::pdf::document::{self, verify_output};
use crate::pdf::images::embed_image;
use crate::pdf::ops::{require_one, Staged};
use crate::pdf::ranges;
use crate::services::context::JobContext;
use crate::services::engines::{self, Engine};

// ---------------------------------------------------------------- images → PDF

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PageSize {
    /// Each page is the size of its image (at 96 pixels per inch).
    #[default]
    Fit,
    A4,
    Letter,
    Legal,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    /// Portrait or landscape to match each image.
    #[default]
    Auto,
    Portrait,
    Landscape,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Margin {
    #[default]
    None,
    Small,
    Large,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ImagesToPdfOptions {
    pub page_size: PageSize,
    pub orientation: Orientation,
    pub margin: Margin,
}

/// Page size in points for an image of `w`×`h` pixels.
pub fn page_dimensions(opts: &ImagesToPdfOptions, w: f64, h: f64) -> (f64, f64) {
    let (pw, ph) = match opts.page_size {
        PageSize::Fit => {
            let m = margin_points(opts.margin) * 2.0;
            return (w * 0.75 + m, h * 0.75 + m);
        }
        PageSize::A4 => (595.276, 841.89),
        PageSize::Letter => (612.0, 792.0),
        PageSize::Legal => (612.0, 1008.0),
    };
    let landscape = match opts.orientation {
        Orientation::Auto => w > h,
        Orientation::Portrait => false,
        Orientation::Landscape => true,
    };
    if landscape {
        (ph, pw)
    } else {
        (pw, ph)
    }
}

fn margin_points(m: Margin) -> f64 {
    match m {
        Margin::None => 0.0,
        Margin::Small => 18.0,
        Margin::Large => 48.0,
    }
}

pub fn images_to_pdf(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: ImagesToPdfOptions,
) -> AppResult<Staged> {
    if inputs.is_empty() {
        return Err(AppError::invalid("Add at least one image."));
    }
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(inputs.len());
    for (i, path) in inputs.iter().enumerate() {
        ctx.check()?;
        ctx.progress_file(
            i as f64 / (inputs.len() + 1) as f64,
            format!("Adding image {} of {}", i + 1, inputs.len()),
            display_name(path),
        );
        validate_image_input(path)?;
        let image = embed_image(&mut doc, path)?;
        let (iw, ih) = (image.width as f64, image.height as f64);
        let (pw, ph) = page_dimensions(&opts, iw, ih);
        let m = margin_points(opts.margin);
        let (bw, bh) = ((pw - 2.0 * m).max(1.0), (ph - 2.0 * m).max(1.0));
        let scale = (bw / iw).min(bh / ih);
        let (dw, dh) = (iw * scale, ih * scale);
        let (x, y) = ((pw - dw) / 2.0, (ph - dh) / 2.0);
        let content = format!("q\n{}Q\n", image.draw_ops("Im1", x, y, dw, dh));
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(pw as f32), Object::Real(ph as f32)],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im1" => image.id } },
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let info = doc.add_object(dictionary! { "Producer" => Object::string_literal("PaperFlow") });
    doc.trailer.set("Info", info);

    ctx.progress(inputs.len() as f64 / (inputs.len() + 1) as f64, "Saving");
    let path = ctx.temp.join("images.pdf");
    document::save(&mut doc, &path)?;
    let mut staged = Staged::single(path.clone(), file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(verify_output(&path, Some(inputs.len()))?);
    Ok(staged)
}

// ---------------------------------------------------------------- PDF → images

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImageFormat {
    #[default]
    Png,
    Jpeg,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PdfToImagesOptions {
    pub format: ImageFormat,
    pub dpi: u32,
    pub pages: String,
    /// JPEG quality, 1-100.
    pub quality: u8,
}

impl Default for PdfToImagesOptions {
    fn default() -> Self {
        Self { format: ImageFormat::Png, dpi: 150, pages: String::new(), quality: 90 }
    }
}

fn base_name(output_name: &str, input: &std::path::Path) -> String {
    let fallback = stem(input);
    let base = sanitize_file_name(if output_name.trim().is_empty() { &fallback } else { output_name });
    base.strip_suffix(".pdf").map(str::to_string).unwrap_or(base)
}

pub fn pdf_to_images(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: PdfToImagesOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let dpi = opts.dpi.clamp(36, 1200);
    let doc = document::load(input)?;
    let total = doc.get_pages().len() as u32;
    drop(doc);
    let pages = ranges::parse_pages(&opts.pages, total)?;
    let base = base_name(output_name, input);
    let out_dir = ctx.temp.subdir("images")?;
    let source = ctx.stage_input(input)?;
    let (flag, ext) = match opts.format {
        ImageFormat::Png => ("-png", "png"),
        ImageFormat::Jpeg => ("-jpeg", "jpg"),
    };
    let width = total.to_string().len();
    let mut staged = Staged { folder: Some(format!("{base} - images")), ..Default::default() };
    for (i, page) in pages.iter().enumerate() {
        ctx.progress_file(
            i as f64 / pages.len() as f64,
            format!("Rendering page {page} ({} of {})", i + 1, pages.len()),
            display_name(input),
        );
        let stem = format!("{base} - page {:0width$}", page);
        let target = out_dir.join(format!("p{page}"));
        let mut args: Vec<String> = vec![
            flag.into(),
            "-r".into(),
            dpi.to_string(),
            "-f".into(),
            page.to_string(),
            "-l".into(),
            page.to_string(),
            "-singlefile".into(),
        ];
        if opts.format == ImageFormat::Jpeg {
            args.push("-jpegopt".into());
            args.push(format!("quality={}", opts.quality.clamp(1, 100)));
        }
        args.push(source.display().to_string());
        args.push(target.display().to_string());
        let out = engines::run(engines::command(Engine::Pdftoppm)?, &args, None, &ctx.cancel, |_| {})?;
        let produced = target.with_extension(ext);
        if !out.success() || !produced.exists() {
            return Err(AppError::processing(format!("Page {page} could not be rendered."))
                .with_detail(out.stderr_tail()));
        }
        staged.files.push((produced, format!("{stem}.{ext}")));
    }
    Ok(staged)
}

// ---------------------------------------------------------------- PDF → text

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PdfToTextOptions {
    pub pages: String,
    /// Keep the physical layout (columns, alignment) instead of reading order.
    pub layout: bool,
}

impl Default for PdfToTextOptions {
    fn default() -> Self {
        Self { pages: String::new(), layout: true }
    }
}

pub fn pdf_to_text(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: PdfToTextOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let doc = document::load(input)?;
    let total = doc.get_pages().len() as u32;
    drop(doc);
    let pages = ranges::parse_pages(&opts.pages, total)?;
    let source = ctx.stage_input(input)?;
    let mut text = String::new();
    let mut empty_pages = 0;
    for (i, page) in pages.iter().enumerate() {
        ctx.progress_file(i as f64 / pages.len() as f64, format!("Reading page {page}"), display_name(input));
        let mut args: Vec<String> =
            vec!["-enc".into(), "UTF-8".into(), "-f".into(), page.to_string(), "-l".into(), page.to_string()];
        if opts.layout {
            args.push("-layout".into());
        }
        let target = ctx.temp.join(format!("page-{page}.txt"));
        args.push(source.display().to_string());
        args.push(target.display().to_string());
        let out = engines::run(engines::command(Engine::Pdftotext)?, &args, None, &ctx.cancel, |_| {})?;
        if !out.success() {
            return Err(AppError::processing(format!("Text could not be read from page {page}."))
                .with_detail(out.stderr_tail()));
        }
        let page_text = fs::read_to_string(&target).unwrap_or_default();
        let page_text = page_text.trim_end_matches(['\u{c}', '\n']);
        if page_text.trim().is_empty() {
            empty_pages += 1;
        }
        if i > 0 {
            text.push('\u{c}');
        }
        text.push_str(page_text);
        text.push('\n');
    }
    let mut staged = Staged::default();
    if empty_pages == pages.len() {
        staged.warn("No text was found. The document may be scanned; run OCR PDF on it first.");
    } else if empty_pages > 0 {
        staged.warn(format!("{empty_pages} page(s) had no text. They may be scanned images."));
    }
    let path = ctx.temp.join("text.txt");
    fs::write(&path, text)?;
    let name = file_name_with_extension(output_name.strip_suffix(".pdf").unwrap_or(output_name), "txt");
    staged.files.push((path, name));
    Ok(staged)
}

// ---------------------------------------------------------------- extract images

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractImagesOptions {
    pub pages: String,
}

pub fn extract_images(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: ExtractImagesOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let doc = document::load(input)?;
    let total = doc.get_pages().len() as u32;
    drop(doc);
    let pages = ranges::parse_pages(&opts.pages, total)?;
    let base = base_name(output_name, input);
    let out_dir = ctx.temp.subdir("extracted")?;
    let source = ctx.stage_input(input)?;
    for (i, page) in pages.iter().enumerate() {
        ctx.progress_file(
            i as f64 / pages.len() as f64,
            format!("Scanning page {page}"),
            display_name(input),
        );
        let args: Vec<String> = vec![
            "-all".into(),
            "-p".into(),
            "-f".into(),
            page.to_string(),
            "-l".into(),
            page.to_string(),
            source.display().to_string(),
            out_dir.join("img").display().to_string(),
        ];
        let out = engines::run(engines::command(Engine::Pdfimages)?, &args, None, &ctx.cancel, |_| {})?;
        if !out.success() {
            return Err(AppError::processing(format!("Images could not be read from page {page}."))
                .with_detail(out.stderr_tail()));
        }
    }
    let mut produced: Vec<PathBuf> = fs::read_dir(&out_dir)?.flatten().map(|e| e.path()).collect();
    produced.sort();
    let mut staged = Staged { folder: Some(format!("{base} - images")), ..Default::default() };
    if produced.is_empty() {
        return Err(AppError::invalid("No embedded images were found in the selected pages.")
            .with_hint("To save whole pages as pictures, use PDF to Images."));
    }
    for (n, path) in produced.into_iter().enumerate() {
        // pdfimages names files img-PPP-NNN.ext (page, image number).
        let file = display_name(&path);
        let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "bin".into());
        let page = file.split('-').nth(1).and_then(|p| p.parse::<u32>().ok());
        let name = match page {
            Some(p) => format!("{base} - page {p} - image {}.{ext}", n + 1),
            None => format!("{base} - image {}.{ext}", n + 1),
        };
        staged.files.push((path, name));
    }
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_sizes() {
        let o = ImagesToPdfOptions { page_size: PageSize::A4, ..Default::default() };
        assert_eq!(page_dimensions(&o, 100.0, 50.0), (841.89, 595.276));
        assert_eq!(page_dimensions(&o, 50.0, 100.0), (595.276, 841.89));
        let o = ImagesToPdfOptions {
            page_size: PageSize::Letter,
            orientation: Orientation::Portrait,
            ..Default::default()
        };
        assert_eq!(page_dimensions(&o, 100.0, 50.0), (612.0, 792.0));
        let o = ImagesToPdfOptions { page_size: PageSize::Fit, margin: Margin::Small, ..Default::default() };
        assert_eq!(page_dimensions(&o, 400.0, 200.0), (336.0, 186.0));
    }

    #[test]
    fn builds_pdf_from_images() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        image::RgbImage::from_pixel(40, 20, image::Rgb([10, 200, 10])).save(&a).unwrap();
        let b = dir.path().join("b.jpg");
        image::RgbImage::from_pixel(20, 40, image::Rgb([200, 10, 10])).save(&b).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let opts =
            ImagesToPdfOptions { page_size: PageSize::A4, margin: Margin::Small, ..Default::default() };
        let s = images_to_pdf(&ctx, &[a, b], "photos", opts).unwrap();
        assert_eq!(s.files[0].1, "photos.pdf");
        let doc = Document::load(&s.files[0].0).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        let first = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
        let mb = first.get(b"MediaBox").unwrap().as_array().unwrap();
        assert!(
            mb[2].as_float().unwrap() > mb[3].as_float().unwrap(),
            "landscape image gets a landscape page"
        );
    }

    #[test]
    fn rejects_non_image_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        fs::write(&f, b"hello").unwrap();
        let ctx = JobContext::for_tests(dir.path());
        assert!(images_to_pdf(&ctx, &[f], "x", ImagesToPdfOptions::default()).is_err());
        assert!(images_to_pdf(&ctx, &[], "x", ImagesToPdfOptions::default()).is_err());
    }
}
