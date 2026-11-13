//! Watermarks and page numbers: content drawn on top of existing pages.

use std::path::PathBuf;

use lopdf::{dictionary, Object};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension, validate_image_input};
use crate::pdf::document::{self, verify_output};
use crate::pdf::fonts::{encode_win_ansi, hex_string, parse_hex_color, StandardFont};
use crate::pdf::geometry::{fmt, fmt_matrix, page_frame};
use crate::pdf::images::embed_image;
use crate::pdf::ops::{require_one, Staged};
use crate::pdf::overlay::{self, Overlay};
use crate::pdf::ranges;
use crate::services::context::JobContext;

const FONT_NAME: &str = "PaperFlowF1";
const GS_NAME: &str = "PaperFlowGS1";
const IMAGE_NAME: &str = "PaperFlowIm1";

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    #[default]
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Position {
    /// Horizontal and vertical alignment: -1 start, 0 centre, 1 end.
    fn alignment(self) -> (i8, i8) {
        use Position::*;
        match self {
            TopLeft => (-1, 1),
            TopCenter => (0, 1),
            TopRight => (1, 1),
            MiddleLeft => (-1, 0),
            Center => (0, 0),
            MiddleRight => (1, 0),
            BottomLeft => (-1, -1),
            BottomCenter => (0, -1),
            BottomRight => (1, -1),
        }
    }

    /// Centre point for a box of half-extents (ex, ey) on a page of
    /// visual size (vw, vh), keeping `margin` from the edges.
    fn anchor(self, vw: f64, vh: f64, ex: f64, ey: f64, margin: f64) -> (f64, f64) {
        let (h, v) = self.alignment();
        let x = match h {
            -1 => margin + ex,
            1 => vw - margin - ex,
            _ => vw / 2.0,
        };
        let y = match v {
            -1 => margin + ey,
            1 => vh - margin - ey,
            _ => vh / 2.0,
        };
        (x, y)
    }
}

fn color_or_default(input: &str, default: [f64; 3]) -> AppResult<[f64; 3]> {
    if input.trim().is_empty() {
        return Ok(default);
    }
    parse_hex_color(input).ok_or_else(|| AppError::invalid("The colour must look like #RRGGBB."))
}

/// Rotated bounding-box half-extents of a w×h box turned by `deg`.
fn half_extents(w: f64, h: f64, deg: f64) -> (f64, f64) {
    let (s, c) = deg.to_radians().sin_cos();
    ((w * c.abs() + h * s.abs()) / 2.0, (w * s.abs() + h * c.abs()) / 2.0)
}

fn rotation_matrix(deg: f64, x: f64, y: f64) -> [f64; 6] {
    let (s, c) = deg.to_radians().sin_cos();
    [c, s, -s, c, x, y]
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WatermarkKind {
    #[default]
    Text,
    Image,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatermarkOptions {
    pub kind: WatermarkKind,
    pub text: String,
    pub font: StandardFont,
    pub font_size: f64,
    pub color: String,
    pub image_path: Option<String>,
    /// Image width as a fraction of the page width.
    pub image_scale: f64,
    pub position: Position,
    /// 0 (invisible) to 1 (opaque).
    pub opacity: f64,
    /// Anticlockwise degrees.
    pub rotation: f64,
    pub pages: String,
    pub margin: f64,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            kind: WatermarkKind::Text,
            text: String::new(),
            font: StandardFont::HelveticaBold,
            font_size: 48.0,
            color: "#D94C4C".into(),
            image_path: None,
            image_scale: 0.4,
            position: Position::Center,
            opacity: 0.3,
            rotation: 45.0,
            pages: String::new(),
            margin: 36.0,
        }
    }
}

pub fn watermark(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: WatermarkOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let opacity = opts.opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return Err(AppError::invalid("Opacity must be above 0%, or the watermark would be invisible."));
    }
    ctx.progress_file(0.05, "Reading document", display_name(input));
    let mut doc = document::load(input)?;
    let mut staged = Staged::default();

    let gs =
        doc.add_object(dictionary! { "Type" => "ExtGState", "ca" => opacity as f32, "CA" => opacity as f32 });
    let mut base =
        Overlay { ext_gstates: vec![(GS_NAME.into(), Object::Reference(gs))], ..Default::default() };

    enum Mark {
        Text { encoded: Vec<u8>, size: f64, width: f64, height: f64, color: [f64; 3] },
        Image { image: crate::pdf::images::EmbeddedImage, scale: f64 },
    }
    let mark = match opts.kind {
        WatermarkKind::Text => {
            let text = opts.text.trim();
            if text.is_empty() {
                return Err(AppError::invalid("Enter the watermark text."));
            }
            let size = opts.font_size.clamp(4.0, 400.0);
            let (encoded, lossy) = encode_win_ansi(text, opts.font);
            if lossy {
                staged.warn(
                    "Some characters are not available in the standard PDF fonts and were replaced with “?”.",
                );
            }
            let font = doc.add_object(opts.font.dictionary());
            base.fonts.push((FONT_NAME.into(), Object::Reference(font)));
            let width = opts.font.text_width(&encoded, size);
            Mark::Text {
                encoded,
                size,
                width,
                height: size * opts.font.cap_height(),
                color: color_or_default(&opts.color, [0.85, 0.3, 0.3])?,
            }
        }
        WatermarkKind::Image => {
            let path = opts
                .image_path
                .as_deref()
                .filter(|p| !p.is_empty())
                .ok_or_else(|| AppError::invalid("Choose an image for the watermark."))?;
            let path = PathBuf::from(path);
            validate_image_input(&path)?;
            let image = embed_image(&mut doc, &path)?;
            base.xobjects.push((IMAGE_NAME.into(), Object::Reference(image.id)));
            Mark::Image { image, scale: opts.image_scale.clamp(0.02, 1.0) }
        }
    };

    let page_ids = doc.get_pages();
    let selected = ranges::parse_pages(&opts.pages, page_ids.len() as u32)?;
    for (i, n) in selected.iter().enumerate() {
        ctx.check()?;
        if i % 10 == 0 {
            ctx.progress(0.1 + 0.75 * i as f64 / selected.len() as f64, format!("Stamping page {n}"));
        }
        let id = page_ids[n];
        let frame = page_frame(&doc, id);
        let (vw, vh) = (frame.visual_width(), frame.visual_height());
        let mut ops = format!("q\n{} cm\n/{GS_NAME} gs\n", fmt_matrix(&frame.visual_matrix()));
        match &mark {
            Mark::Text { encoded, size, width, height, color } => {
                let (ex, ey) = half_extents(*width, *height, opts.rotation);
                let (ax, ay) = opts.position.anchor(vw, vh, ex, ey, opts.margin);
                ops.push_str(&format!(
                    "{} cm\n{} {} {} rg\nBT\n/{FONT_NAME} {} Tf\n{} {} Td\n{} Tj\nET\n",
                    fmt_matrix(&rotation_matrix(opts.rotation, ax, ay)),
                    fmt(color[0]),
                    fmt(color[1]),
                    fmt(color[2]),
                    fmt(*size),
                    fmt(-width / 2.0),
                    fmt(-height / 2.0),
                    hex_string(encoded)
                ));
            }
            Mark::Image { image, scale } => {
                let w = vw * scale;
                let h = w * image.height as f64 / image.width as f64;
                let (ex, ey) = half_extents(w, h, opts.rotation);
                let (ax, ay) = opts.position.anchor(vw, vh, ex, ey, opts.margin);
                ops.push_str(&format!("{} cm\n", fmt_matrix(&rotation_matrix(opts.rotation, ax, ay))));
                ops.push_str(&image.draw_ops(IMAGE_NAME, -w / 2.0, -h / 2.0, w, h));
            }
        }
        ops.push_str("Q\n");
        let overlay = Overlay {
            content: ops.into_bytes(),
            fonts: base.fonts.clone(),
            ext_gstates: base.ext_gstates.clone(),
            xobjects: base.xobjects.clone(),
        };
        overlay::apply(&mut doc, id, &overlay)?;
    }

    ctx.progress(0.9, "Saving");
    let path = ctx.temp.join("watermarked.pdf");
    document::save(&mut doc, &path)?;
    staged.page_count = Some(verify_output(&path, Some(page_ids.len()))?);
    staged.files.push((path, file_name_with_extension(output_name, "pdf")));
    Ok(staged)
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum NumberFormat {
    /// "3"
    #[default]
    Number,
    /// "3 of 12"
    NumberOfTotal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageNumberOptions {
    pub position: Position,
    pub font: StandardFont,
    pub font_size: f64,
    pub color: String,
    pub start: i64,
    pub prefix: String,
    pub suffix: String,
    pub format: NumberFormat,
    /// Pages that receive a number; empty means all.
    pub pages: String,
    pub margin: f64,
}

impl Default for PageNumberOptions {
    fn default() -> Self {
        Self {
            position: Position::BottomCenter,
            font: StandardFont::Helvetica,
            font_size: 11.0,
            color: "#20242C".into(),
            start: 1,
            prefix: String::new(),
            suffix: String::new(),
            format: NumberFormat::Number,
            pages: String::new(),
            margin: 28.0,
        }
    }
}

pub fn page_number_label(opts: &PageNumberOptions, number: i64, last: i64) -> String {
    let core = match opts.format {
        NumberFormat::Number => number.to_string(),
        NumberFormat::NumberOfTotal => format!("{number} of {last}"),
    };
    format!("{}{}{}", opts.prefix, core, opts.suffix)
}

pub fn page_numbers(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: PageNumberOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    ctx.progress_file(0.05, "Reading document", display_name(input));
    let mut doc = document::load(input)?;
    let mut staged = Staged::default();
    let color = color_or_default(&opts.color, [0.0, 0.0, 0.0])?;
    let size = opts.font_size.clamp(4.0, 144.0);
    let font = doc.add_object(opts.font.dictionary());
    let fonts = vec![(FONT_NAME.to_string(), Object::Reference(font))];

    let page_ids = doc.get_pages();
    let selected = ranges::parse_pages(&opts.pages, page_ids.len() as u32)?;
    let last = opts.start + selected.len() as i64 - 1;
    let mut lossy_any = false;
    for (i, n) in selected.iter().enumerate() {
        ctx.check()?;
        if i % 10 == 0 {
            ctx.progress(0.1 + 0.75 * i as f64 / selected.len() as f64, format!("Numbering page {n}"));
        }
        let id = page_ids[n];
        let label = page_number_label(&opts, opts.start + i as i64, last);
        let (encoded, lossy) = encode_win_ansi(&label, opts.font);
        lossy_any |= lossy;
        let width = opts.font.text_width(&encoded, size);
        let height = size * opts.font.cap_height();
        let frame = page_frame(&doc, id);
        let (ax, ay) = opts.position.anchor(
            frame.visual_width(),
            frame.visual_height(),
            width / 2.0,
            height / 2.0,
            opts.margin,
        );
        let ops = format!(
            "q\n{} cm\n{} {} {} rg\nBT\n/{FONT_NAME} {} Tf\n{} {} Td\n{} Tj\nET\nQ\n",
            fmt_matrix(&frame.visual_matrix()),
            fmt(color[0]),
            fmt(color[1]),
            fmt(color[2]),
            fmt(size),
            fmt(ax - width / 2.0),
            fmt(ay - height / 2.0),
            hex_string(&encoded)
        );
        overlay::apply(
            &mut doc,
            id,
            &Overlay { content: ops.into_bytes(), fonts: fonts.clone(), ..Default::default() },
        )?;
    }
    if lossy_any {
        staged
            .warn("Some characters are not available in the standard PDF fonts and were replaced with “?”.");
    }
    ctx.progress(0.9, "Saving");
    let path = ctx.temp.join("numbered.pdf");
    document::save(&mut doc, &path)?;
    staged.page_count = Some(verify_output(&path, Some(page_ids.len()))?);
    staged.files.push((path, file_name_with_extension(output_name, "pdf")));
    Ok(staged)
}

/// Text of each page, for tests and diagnostics.
#[cfg(test)]
fn texts(path: &std::path::Path) -> Vec<String> {
    let doc = lopdf::Document::load(path).unwrap();
    doc.get_pages().keys().map(|n| doc.extract_text(&[*n]).unwrap_or_default()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::assemble::tests::sample_doc;

    #[test]
    fn anchors_respect_margins() {
        let (x, y) = Position::BottomRight.anchor(600.0, 800.0, 10.0, 5.0, 20.0);
        assert_eq!((x, y), (570.0, 25.0));
        let (x, y) = Position::Center.anchor(600.0, 800.0, 10.0, 5.0, 20.0);
        assert_eq!((x, y), (300.0, 400.0));
        let (ex, ey) = half_extents(100.0, 0.0, 90.0);
        assert!(ex.abs() < 1e-9 && (ey - 50.0).abs() < 1e-9);
    }

    #[test]
    fn labels() {
        let mut o = PageNumberOptions { prefix: "Page ".into(), ..Default::default() };
        assert_eq!(page_number_label(&o, 3, 9), "Page 3");
        o.format = NumberFormat::NumberOfTotal;
        o.suffix = ".".into();
        assert_eq!(page_number_label(&o, 3, 9), "Page 3 of 9.");
    }

    #[test]
    fn stamps_text_on_selected_pages() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        sample_doc(3, "A").save(&input).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let opts =
            WatermarkOptions { text: "CONFIDENTIAL".into(), pages: "1,3".into(), ..Default::default() };
        let s = watermark(&ctx, std::slice::from_ref(&input), "w", opts).unwrap();
        let t = texts(&s.files[0].0);
        assert!(t[0].contains("CONFIDENTIAL") && t[0].contains("A Page 1"));
        assert!(!t[1].contains("CONFIDENTIAL") && t[1].contains("A Page 2"));
        assert!(t[2].contains("CONFIDENTIAL"));

        let empty = WatermarkOptions { text: "  ".into(), ..Default::default() };
        assert!(watermark(&ctx, &[input], "w", empty).is_err());
    }

    #[test]
    fn stamps_image_watermark() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        sample_doc(2, "A").save(&input).unwrap();
        let logo = dir.path().join("logo.png");
        image::RgbaImage::from_pixel(10, 5, image::Rgba([0, 0, 0, 128])).save(&logo).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let opts = WatermarkOptions {
            kind: WatermarkKind::Image,
            image_path: Some(logo.display().to_string()),
            ..Default::default()
        };
        let s = watermark(&ctx, &[input], "w", opts).unwrap();
        let doc = lopdf::Document::load(&s.files[0].0).unwrap();
        let page = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
        let xobjects =
            page.get(b"Resources").unwrap().as_dict().unwrap().get(b"XObject").unwrap().as_dict().unwrap();
        assert!(xobjects.has(IMAGE_NAME.as_bytes()));
    }

    #[test]
    fn numbers_pages() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        sample_doc(3, "A").save(&input).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let opts = PageNumberOptions {
            start: 5,
            prefix: "p. ".into(),
            format: NumberFormat::NumberOfTotal,
            pages: "2-3".into(),
            ..Default::default()
        };
        let s = page_numbers(&ctx, &[input], "n", opts).unwrap();
        let t = texts(&s.files[0].0);
        assert!(!t[0].contains("p. "));
        assert!(t[1].contains("p. 5 of 6"));
        assert!(t[2].contains("p. 6 of 6"));
    }
}
