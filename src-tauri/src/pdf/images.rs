//! Embedding raster images as PDF image XObjects.
//!
//! JPEG files are embedded byte-for-byte (no recompression, no quality loss);
//! their EXIF rotation is applied when the image is drawn. Other formats are
//! decoded and stored losslessly with Flate compression, with transparency
//! kept as a soft mask.

use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::display_name;
use crate::pdf::geometry::fmt;

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedImage {
    pub id: ObjectId,
    /// Size as the image should be displayed (after EXIF rotation), in pixels.
    pub width: u32,
    pub height: u32,
    /// Clockwise quarter turns still to apply when drawing (0..=3).
    pub quarter_turns: u8,
}

impl EmbeddedImage {
    /// Operators drawing the image upright into the rectangle
    /// (x, y, w, h). Must be wrapped in q/Q by the caller.
    pub fn draw_ops(&self, name: &str, x: f64, y: f64, w: f64, h: f64) -> String {
        let m = match self.quarter_turns {
            1 => [0.0, -h, w, 0.0, x, y + h],
            2 => [-w, 0.0, 0.0, -h, x + w, y + h],
            3 => [0.0, h, -w, 0.0, x + w, y],
            _ => [w, 0.0, 0.0, h, x, y],
        };
        format!(
            "{} {} {} {} {} {} cm /{} Do\n",
            fmt(m[0]),
            fmt(m[1]),
            fmt(m[2]),
            fmt(m[3]),
            fmt(m[4]),
            fmt(m[5]),
            name
        )
    }
}

struct JpegInfo {
    width: u32,
    height: u32,
    components: u8,
    adobe: bool,
}

/// Reads the frame header of a baseline or progressive 8-bit JPEG.
fn jpeg_info(data: &[u8]) -> Option<JpegInfo> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    let mut adobe = false;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if marker == 0xEE && data.get(i + 4..i + 9) == Some(b"Adobe") {
            adobe = true;
        }
        let is_sof = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_sof {
            let seg = data.get(i + 4..i + 2 + len)?;
            if seg.len() < 6 || seg[0] != 8 {
                return None;
            }
            // Only baseline (C0), extended (C1) and progressive (C2) are widely supported.
            if !matches!(marker, 0xC0..=0xC2) {
                return None;
            }
            let height = u16::from_be_bytes([seg[1], seg[2]]) as u32;
            let width = u16::from_be_bytes([seg[3], seg[4]]) as u32;
            let components = seg[5];
            if width == 0 || height == 0 || !matches!(components, 1 | 3 | 4) {
                return None;
            }
            return Some(JpegInfo { width, height, components, adobe });
        }
        if marker == 0xDA {
            return None;
        }
        i += 2 + len;
    }
    None
}

fn deflate(data: &[u8]) -> AppResult<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::with_capacity(data.len() / 2), Compression::default());
    enc.write_all(data)?;
    Ok(enc.finish()?)
}

fn quarter_turns(o: Orientation) -> Option<u8> {
    match o {
        Orientation::NoTransforms => Some(0),
        Orientation::Rotate90 => Some(1),
        Orientation::Rotate180 => Some(2),
        Orientation::Rotate270 => Some(3),
        _ => None,
    }
}

/// Adds an image file to the document and returns its XObject.
pub fn embed_image(doc: &mut Document, path: &Path) -> AppResult<EmbeddedImage> {
    let name = display_name(path);
    let bad = |e: String| {
        AppError::invalid(format!("“{name}” could not be read as an image."))
            .with_hint("Use PNG, JPEG, WebP, BMP, TIFF or GIF images.")
            .with_detail(e)
    };
    let data = std::fs::read(path)?;
    let format = image::guess_format(&data).map_err(|e| bad(e.to_string()))?;
    let mut decoder = ImageReader::new(std::io::Cursor::new(&data))
        .with_guessed_format()
        .map_err(|e| bad(e.to_string()))?
        .into_decoder()
        .map_err(|e| bad(e.to_string()))?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    if format == ImageFormat::Jpeg {
        if let (Some(info), Some(turns)) = (jpeg_info(&data), quarter_turns(orientation)) {
            drop(decoder);
            let color_space = match info.components {
                1 => "DeviceGray",
                3 => "DeviceRGB",
                _ => "DeviceCMYK",
            };
            let mut dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => info.width as i64,
                "Height" => info.height as i64,
                "ColorSpace" => color_space,
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            };
            if info.components == 4 && info.adobe {
                // Adobe CMYK JPEGs are stored inverted.
                dict.set(
                    "Decode",
                    vec![1.into(), 0.into(), 1.into(), 0.into(), 1.into(), 0.into(), 1.into(), 0.into()],
                );
            }
            let id = doc.add_object(Stream::new(dict, data).with_compression(false));
            let (w, h) = if turns % 2 == 1 { (info.height, info.width) } else { (info.width, info.height) };
            return Ok(EmbeddedImage { id, width: w, height: h, quarter_turns: turns });
        }
    }

    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| bad(e.to_string()))?;
    img.apply_orientation(orientation);
    let (width, height) = (img.width(), img.height());
    if width == 0 || height == 0 {
        return Err(bad("empty image".into()));
    }

    let has_alpha = img.color().has_alpha();
    let is_gray = !img.color().has_color();
    let mut smask: Option<ObjectId> = None;
    if has_alpha {
        let rgba = img.to_rgba8();
        let alpha: Vec<u8> = rgba.pixels().map(|p| p.0[3]).collect();
        if alpha.iter().any(|&a| a != 255) {
            let mask = Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image",
                    "Width" => width as i64, "Height" => height as i64,
                    "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                    "Filter" => "FlateDecode",
                },
                deflate(&alpha)?,
            )
            .with_compression(false);
            smask = Some(doc.add_object(mask));
        }
    }
    let (pixels, color_space) = if is_gray {
        (img.to_luma8().into_raw(), "DeviceGray")
    } else {
        (img.to_rgb8().into_raw(), "DeviceRGB")
    };
    let mut dict = dictionary! {
        "Type" => "XObject", "Subtype" => "Image",
        "Width" => width as i64, "Height" => height as i64,
        "ColorSpace" => color_space, "BitsPerComponent" => 8,
        "Filter" => "FlateDecode",
    };
    if let Some(mask) = smask {
        dict.set("SMask", Object::Reference(mask));
    }
    let id = doc.add_object(Stream::new(dict, deflate(&pixels)?).with_compression(false));
    Ok(EmbeddedImage { id, width, height, quarter_turns: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_jpeg_header() {
        let img = image::RgbImage::from_pixel(7, 3, image::Rgb([200, 10, 10]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Jpeg)
            .unwrap();
        let info = jpeg_info(&buf).unwrap();
        assert_eq!((info.width, info.height, info.components), (7, 3, 3));
        assert!(jpeg_info(b"not a jpeg").is_none());
    }

    #[test]
    fn embeds_png_with_alpha_and_jpeg_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("a.png");
        let mut rgba = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 0, 255, 255]));
        rgba.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
        rgba.save(&png).unwrap();
        let jpg = dir.path().join("b.jpg");
        image::RgbImage::from_pixel(5, 6, image::Rgb([1, 2, 3])).save(&jpg).unwrap();

        let mut doc = Document::with_version("1.7");
        let a = embed_image(&mut doc, &png).unwrap();
        assert_eq!((a.width, a.height, a.quarter_turns), (4, 2, 0));
        let stream = doc.get_object(a.id).unwrap().as_stream().unwrap();
        assert!(stream.dict.has(b"SMask"));

        let b = embed_image(&mut doc, &jpg).unwrap();
        let stream = doc.get_object(b.id).unwrap().as_stream().unwrap();
        assert_eq!(stream.dict.get(b"Filter").unwrap().as_name().unwrap(), b"DCTDecode");
        assert_eq!(stream.content, std::fs::read(&jpg).unwrap());
    }

    #[test]
    fn rejects_non_images() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x.png");
        std::fs::write(&f, b"definitely not an image").unwrap();
        let mut doc = Document::with_version("1.7");
        assert!(embed_image(&mut doc, &f).is_err());
    }
}
