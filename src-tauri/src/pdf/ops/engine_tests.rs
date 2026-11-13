//! Integration tests that drive the real external engines on generated PDF
//! fixtures and check the actual output. Each test is skipped (with a note on
//! stderr) when its engine is not installed. To run them all during
//! development, see `scripts/dev-engines.sh`.

use std::fs;
use std::path::{Path, PathBuf};

use lopdf::Document;

use crate::error::ErrorKind;
use crate::pdf::assemble::tests::sample_doc;
use crate::pdf::ops::compress::{compress, CompressOptions, Preset};
use crate::pdf::ops::convert::*;
use crate::pdf::ops::ocr::{installed_languages, ocr, OcrOptions};
use crate::pdf::ops::security::*;
use crate::services::context::JobContext;
use crate::services::engines::{locate, Engine};

fn have(engines: &[Engine]) -> bool {
    let missing: Vec<_> = engines.iter().filter(|e| locate(**e).is_none()).collect();
    if !missing.is_empty() {
        eprintln!("skipping: engines not installed: {missing:?}");
    }
    missing.is_empty()
}

fn sample(dir: &Path, pages: u32) -> PathBuf {
    let p = dir.join("sample.pdf");
    sample_doc(pages, "Fixture").save(&p).unwrap();
    p
}

/// A PDF with a large, noisy photo-like image, which compresses well.
fn image_pdf(dir: &Path) -> PathBuf {
    let img = dir.join("photo.png");
    let mut buf = image::RgbImage::new(1600, 1200);
    for (x, y, p) in buf.enumerate_pixels_mut() {
        let n = ((x * 7919 + y * 104729) % 251) as u8;
        *p = image::Rgb([(x / 7) as u8 ^ n, (y / 5) as u8, n]);
    }
    buf.save(&img).unwrap();
    let ctx = JobContext::for_tests(dir);
    let staged = images_to_pdf(&ctx, &[img], "photo", ImagesToPdfOptions::default()).unwrap();
    let out = dir.join("photo.pdf");
    fs::copy(&staged.files[0].0, &out).unwrap();
    out
}

#[test]
fn compresses_and_keeps_pages() {
    if !have(&[Engine::Ghostscript]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = image_pdf(dir.path());
    let ctx = JobContext::for_tests(dir.path());
    let s = compress(&ctx, &[input.clone()], "small", CompressOptions { preset: Preset::Extreme, ..Default::default() }).unwrap();
    let before = fs::metadata(&input).unwrap().len();
    let after = fs::metadata(&s.files[0].0).unwrap().len();
    assert!(after < before, "{after} < {before}");
    assert_eq!(Document::load(&s.files[0].0).unwrap().get_pages().len(), 1);
}

#[test]
fn renders_pages_to_images() {
    if !have(&[Engine::Pdftoppm]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = sample(dir.path(), 3);
    let ctx = JobContext::for_tests(dir.path());
    let opts = PdfToImagesOptions { format: ImageFormat::Png, dpi: 72, pages: "1,3".into(), ..Default::default() };
    let s = pdf_to_images(&ctx, &[input.clone()], "", opts).unwrap();
    assert_eq!(s.files.len(), 2);
    assert_eq!(s.files[1].1, "sample - page 3.png");
    let img = image::open(&s.files[0].0).unwrap();
    // A4 at 72 dpi.
    assert_eq!((img.width(), img.height()), (595, 842));

    let opts = PdfToImagesOptions { format: ImageFormat::Jpeg, dpi: 36, pages: "2".into(), quality: 80 };
    let s = pdf_to_images(&ctx, &[input], "", opts).unwrap();
    assert!(s.files[0].1.ends_with(".jpg"));
    assert_eq!(image::guess_format(&fs::read(&s.files[0].0).unwrap()).unwrap(), image::ImageFormat::Jpeg);
}

#[test]
fn extracts_text() {
    if !have(&[Engine::Pdftotext]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = sample(dir.path(), 3);
    let ctx = JobContext::for_tests(dir.path());
    let s = pdf_to_text(&ctx, &[input], "out", PdfToTextOptions { pages: "2-3".into(), layout: false }).unwrap();
    assert_eq!(s.files[0].1, "out.txt");
    let text = fs::read_to_string(&s.files[0].0).unwrap();
    assert!(text.contains("Fixture Page 2") && text.contains("Fixture Page 3"));
    assert!(!text.contains("Fixture Page 1"));
    assert!(s.warnings.is_empty());
}

#[test]
fn extracts_embedded_images() {
    if !have(&[Engine::Pdfimages]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = image_pdf(dir.path());
    let ctx = JobContext::for_tests(dir.path());
    let s = extract_images(&ctx, &[input], "photo", ExtractImagesOptions::default()).unwrap();
    assert_eq!(s.files.len(), 1);
    let img = image::open(&s.files[0].0).unwrap();
    assert_eq!((img.width(), img.height()), (1600, 1200));

    let text_only = sample(dir.path(), 1);
    assert!(extract_images(&ctx, &[text_only], "x", ExtractImagesOptions::default()).is_err());
}

#[test]
fn protects_and_unlocks() {
    if !have(&[Engine::Qpdf]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = sample(dir.path(), 2);
    let ctx = JobContext::for_tests(dir.path());
    let opts = ProtectOptions { user_password: "open sesame".into(), printing: PrintPermission::None, ..Default::default() };
    let s = protect(&ctx, &[input.clone()], "locked", opts).unwrap();
    let locked = dir.path().join("locked.pdf");
    fs::copy(&s.files[0].0, &locked).unwrap();
    assert!(!s.warnings.is_empty(), "random owner password is reported");

    // The protected file cannot be opened without the password...
    let err = crate::pdf::document::load(&locked).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Encrypted);
    // ...or be protected twice.
    let again = protect(&ctx, &[locked.clone()], "x", ProtectOptions { user_password: "a".into(), ..Default::default() });
    assert!(again.is_err());

    let wrong = unlock(&ctx, &[locked.clone()], "u", UnlockOptions { password: "nope".into() }).unwrap_err();
    assert_eq!(wrong.kind, ErrorKind::WrongPassword);

    let s = unlock(&ctx, &[locked], "u", UnlockOptions { password: "open sesame".into() }).unwrap();
    let doc = crate::pdf::document::load(&s.files[0].0).unwrap();
    assert_eq!(doc.get_pages().len(), 2);

    let plain = unlock(&ctx, &[input], "u", UnlockOptions { password: "x".into() });
    assert!(plain.is_err(), "unlocking an unprotected file is refused");
}

#[test]
fn ocr_makes_scans_searchable() {
    if !have(&[Engine::Tesseract, Engine::Pdftoppm, Engine::Pdftotext]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Make a "scan": render a text page to an image, then wrap it in a PDF.
    let input = sample(dir.path(), 1);
    let ctx = JobContext::for_tests(dir.path());
    let rendered = pdf_to_images(&ctx, &[input], "scan", PdfToImagesOptions { dpi: 150, ..Default::default() }).unwrap();
    let scan = images_to_pdf(&ctx, &[rendered.files[0].0.clone()], "scan", ImagesToPdfOptions::default()).unwrap();
    let scan_pdf = dir.path().join("scan.pdf");
    fs::copy(&scan.files[0].0, &scan_pdf).unwrap();

    // Before OCR there is no text.
    let before = pdf_to_text(&ctx, &[scan_pdf.clone()], "t", PdfToTextOptions::default()).unwrap();
    assert!(fs::read_to_string(&before.files[0].0).unwrap().trim().is_empty());

    let langs = installed_languages(&ctx.cancel).unwrap();
    assert!(langs.contains(&"eng".to_string()), "{langs:?}");
    let s = ocr(&ctx, &[scan_pdf], "searchable", OcrOptions { language: "eng".into(), dpi: 200 }).unwrap();
    let searchable = dir.path().join("searchable.pdf");
    fs::copy(&s.files[0].0, &searchable).unwrap();
    let after = pdf_to_text(&ctx, &[searchable], "t", PdfToTextOptions::default()).unwrap();
    let text = fs::read_to_string(&after.files[0].0).unwrap();
    assert!(text.contains("Fixture Page 1"), "OCR text was: {text:?}");
}

#[test]
fn engines_handle_awkward_paths() {
    if !have(&[Engine::Qpdf, Engine::Ghostscript, Engine::Pdftotext]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // A hidden folder and a name that looks like an option.
    let hidden = dir.path().join(".private docs");
    fs::create_dir_all(&hidden).unwrap();
    let input = hidden.join("-report (final).PDF");
    sample_doc(2, "Fixture").save(&input).unwrap();
    let ctx = JobContext::for_tests(dir.path());

    let s = protect(&ctx, &[input.clone()], "p", ProtectOptions { user_password: "x".into(), ..Default::default() }).unwrap();
    assert!(s.files[0].0.exists());
    let s = compress(&ctx, &[input.clone()], "c", CompressOptions::default()).unwrap();
    assert!(s.files[0].0.exists());
    let s = pdf_to_text(&ctx, &[input.clone()], "t", PdfToTextOptions::default()).unwrap();
    assert!(fs::read_to_string(&s.files[0].0).unwrap().contains("Fixture Page 2"));
    assert!(input.exists(), "the original is untouched");
}
