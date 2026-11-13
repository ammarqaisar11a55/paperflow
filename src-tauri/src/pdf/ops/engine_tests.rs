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
    let s = compress(
        &ctx,
        std::slice::from_ref(&input),
        "small",
        CompressOptions { preset: Preset::Extreme, ..Default::default() },
    )
    .unwrap();
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
    let opts =
        PdfToImagesOptions { format: ImageFormat::Png, dpi: 72, pages: "1,3".into(), ..Default::default() };
    let s = pdf_to_images(&ctx, std::slice::from_ref(&input), "", opts).unwrap();
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
    let s =
        pdf_to_text(&ctx, &[input], "out", PdfToTextOptions { pages: "2-3".into(), layout: false }).unwrap();
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
    let opts = ProtectOptions {
        user_password: "open sesame".into(),
        printing: PrintPermission::None,
        ..Default::default()
    };
    let s = protect(&ctx, std::slice::from_ref(&input), "locked", opts).unwrap();
    let locked = dir.path().join("locked.pdf");
    fs::copy(&s.files[0].0, &locked).unwrap();
    assert!(!s.warnings.is_empty(), "random owner password is reported");

    // The protected file cannot be opened without the password...
    let err = crate::pdf::document::load(&locked).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Encrypted);
    // ...or be protected twice.
    let again = protect(
        &ctx,
        std::slice::from_ref(&locked),
        "x",
        ProtectOptions { user_password: "a".into(), ..Default::default() },
    );
    assert!(again.is_err());

    let wrong = unlock(&ctx, std::slice::from_ref(&locked), "u", UnlockOptions { password: "nope".into() })
        .unwrap_err();
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
    let rendered =
        pdf_to_images(&ctx, &[input], "scan", PdfToImagesOptions { dpi: 150, ..Default::default() }).unwrap();
    let scan =
        images_to_pdf(&ctx, &[rendered.files[0].0.clone()], "scan", ImagesToPdfOptions::default()).unwrap();
    let scan_pdf = dir.path().join("scan.pdf");
    fs::copy(&scan.files[0].0, &scan_pdf).unwrap();

    // Before OCR there is no text.
    let before =
        pdf_to_text(&ctx, std::slice::from_ref(&scan_pdf), "t", PdfToTextOptions::default()).unwrap();
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

    let s = protect(
        &ctx,
        std::slice::from_ref(&input),
        "p",
        ProtectOptions { user_password: "x".into(), ..Default::default() },
    )
    .unwrap();
    assert!(s.files[0].0.exists());
    let s = compress(&ctx, std::slice::from_ref(&input), "c", CompressOptions::default()).unwrap();
    assert!(s.files[0].0.exists());
    let s = pdf_to_text(&ctx, std::slice::from_ref(&input), "t", PdfToTextOptions::default()).unwrap();
    assert!(fs::read_to_string(&s.files[0].0).unwrap().contains("Fixture Page 2"));
    assert!(input.exists(), "the original is untouched");
}

/// A PDF with a title, a heading, a two-line paragraph and a bulleted list,
/// set in different sizes of Helvetica.
fn structured_pdf(dir: &Path) -> PathBuf {
    use crate::pdf::fonts::{encode_win_ansi, hex_string, StandardFont};
    use lopdf::{dictionary, Object, Stream};
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let regular = doc.add_object(StandardFont::Helvetica.dictionary());
    let bold = doc.add_object(StandardFont::HelveticaBold.dictionary());
    let lines: [(&str, f64, f64, &str); 7] = [
        ("F2", 26.0, 740.0, "Annual Report"),
        ("F2", 17.0, 690.0, "Overview"),
        ("F1", 11.0, 660.0, "PaperFlow keeps every document on your"),
        ("F1", 11.0, 646.0, "computer and works without a network."),
        ("F1", 11.0, 610.0, "\u{2022} Merge and split"),
        ("F1", 11.0, 596.0, "\u{2022} Compress and protect"),
        ("F1", 11.0, 560.0, "Thank you for reading."),
    ];
    let mut content = String::new();
    for (font, size, y, text) in lines {
        let (bytes, _) = encode_win_ansi(text, StandardFont::Helvetica);
        content.push_str(&format!("BT /{font} {size} Tf 72 {y} Td {} Tj ET\n", hex_string(&bytes)));
    }
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => regular, "F2" => bold } },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let path = dir.join("structured.pdf");
    doc.save(&path).unwrap();
    path
}

#[test]
fn converts_pdf_to_markdown() {
    use crate::pdf::ops::markdown::{pdf_to_markdown, MarkdownOptions};
    if !have(&[Engine::Pdftohtml]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = structured_pdf(dir.path());
    let ctx = JobContext::for_tests(dir.path());
    let s = pdf_to_markdown(&ctx, &[input], "structured", MarkdownOptions::default()).unwrap();
    assert_eq!(s.files[0].1, "structured.md");
    let md = fs::read_to_string(&s.files[0].0).unwrap();
    assert!(md.starts_with("# **Annual Report**") || md.starts_with("# Annual Report"), "{md}");
    assert!(md.contains("\n## Overview\n"), "{md}");
    assert!(
        md.contains("PaperFlow keeps every document on your computer and works without a network."),
        "{md}"
    );
    assert!(md.contains("- Merge and split\n- Compress and protect"), "{md}");
    assert!(md.trim_end().ends_with("Thank you for reading."), "{md}");
}

/// Text of a PDF, via pdftotext.
fn pdf_text(path: &Path) -> String {
    let ctx = JobContext::for_tests(path.parent().unwrap());
    let s = pdf_to_text(&ctx, &[path.to_path_buf()], "t", PdfToTextOptions::default()).unwrap();
    fs::read_to_string(&s.files[0].0).unwrap()
}

fn convert_office(
    name: &str,
    contents: &str,
    kind: crate::pdf::ops::office::OfficeKind,
) -> (PathBuf, tempfile::TempDir) {
    use crate::pdf::ops::office::{office_to_pdf, OfficeOptions};
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join(name);
    fs::write(&input, contents).unwrap();
    let ctx = JobContext::for_tests(dir.path());
    let s = office_to_pdf(&ctx, &[input], "out", kind, OfficeOptions::default()).unwrap();
    assert_eq!(s.files[0].1, "out.pdf");
    let out = dir.path().join("converted.pdf");
    fs::copy(&s.files[0].0, &out).unwrap();
    (out, dir)
}

#[test]
fn converts_office_documents_and_html() {
    use crate::pdf::ops::office::OfficeKind;
    if !have(&[Engine::LibreOffice, Engine::Pdftotext]) {
        return;
    }
    let (pdf, _d1) = convert_office(
        "letter.rtf",
        r"{\rtf1\ansi{\fonttbl{\f0 Helvetica;}}\f0\fs24 Dear reader,\par This letter came from Word.\par}",
        OfficeKind::Word,
    );
    assert!(pdf_text(&pdf).contains("This letter came from Word."));

    let (pdf, _d2) = convert_office("budget.csv", "Item,Cost\nPaper,12\nInk,30\n", OfficeKind::Spreadsheet);
    let text = pdf_text(&pdf);
    assert!(text.contains("Paper") && text.contains("30"), "{text}");

    let slides = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
<office:body><office:presentation>
<draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>First slide title</text:p></draw:text-box></draw:frame></draw:page>
<draw:page draw:name="Two"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="20cm" svg:height="3cm"><draw:text-box><text:p>Second slide title</text:p></draw:text-box></draw:frame></draw:page>
</office:presentation></office:body></office:document>"#;
    let (pdf, _d3) = convert_office("deck.fodp", slides, OfficeKind::Presentation);
    assert_eq!(Document::load(&pdf).unwrap().get_pages().len(), 2, "one page per slide");
    assert!(pdf_text(&pdf).contains("Second slide title"));

    // A remote image must not be fetched; the conversion still succeeds offline.
    let html = r#"<!doctype html><html><head><title>Notes</title></head><body>
<h1>Meeting notes</h1><p>Agreed to ship <b>PaperFlow</b> on Friday.</p>
<img src="https://example.invalid/tracker.png" alt=""></body></html>"#;
    let (pdf, _d4) = convert_office("notes.html", html, OfficeKind::Html);
    let text = pdf_text(&pdf);
    assert!(text.contains("Meeting notes") && text.contains("ship PaperFlow on Friday"), "{text}");
}
