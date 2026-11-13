//! Page geometry: the visible box of a page and the transformation that lets
//! overlays (watermarks, page numbers) be drawn upright in the page's
//! displayed orientation, whatever its /Rotate value.

use lopdf::{Document, Object, ObjectId};

use crate::pdf::document::{inherited, number, page_rotation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageFrame {
    /// Lower-left corner of the visible box in user space.
    pub x0: f64,
    pub y0: f64,
    /// Size of the visible box in user space (before rotation).
    pub width: f64,
    pub height: f64,
    /// Clockwise display rotation: 0, 90, 180 or 270.
    pub rotation: i64,
}

impl PageFrame {
    /// Width as the reader sees the page.
    pub fn visual_width(&self) -> f64 {
        if self.rotation % 180 == 0 {
            self.width
        } else {
            self.height
        }
    }

    /// Height as the reader sees the page.
    pub fn visual_height(&self) -> f64 {
        if self.rotation % 180 == 0 {
            self.height
        } else {
            self.width
        }
    }

    /// Matrix `[a b c d e f]` mapping "visual" coordinates (origin at the
    /// bottom-left of the page as displayed, upright) to user space.
    pub fn visual_matrix(&self) -> [f64; 6] {
        let (x0, y0, w, h) = (self.x0, self.y0, self.width, self.height);
        match self.rotation {
            90 => [0.0, 1.0, -1.0, 0.0, x0 + w, y0],
            180 => [-1.0, 0.0, 0.0, -1.0, x0 + w, y0 + h],
            270 => [0.0, -1.0, 1.0, 0.0, x0, y0 + h],
            _ => [1.0, 0.0, 0.0, 1.0, x0, y0],
        }
    }
}

fn read_box(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<[f64; 4]> {
    let obj = inherited(doc, page_id, key)?;
    let arr = match &obj {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_array().ok()?.clone(),
        Object::Array(a) => a.clone(),
        _ => return None,
    };
    if arr.len() != 4 {
        return None;
    }
    let v: Vec<f64> = arr.iter().filter_map(|o| number(doc, o)).collect();
    if v.len() != 4 {
        return None;
    }
    Some([v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])])
}

/// The visible frame of a page: its CropBox clipped to its MediaBox.
pub fn page_frame(doc: &Document, page_id: ObjectId) -> PageFrame {
    let media = read_box(doc, page_id, b"MediaBox").unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let b = match read_box(doc, page_id, b"CropBox") {
        Some(c) => {
            let clipped = [c[0].max(media[0]), c[1].max(media[1]), c[2].min(media[2]), c[3].min(media[3])];
            if clipped[2] > clipped[0] && clipped[3] > clipped[1] {
                clipped
            } else {
                media
            }
        }
        None => media,
    };
    PageFrame { x0: b[0], y0: b[1], width: b[2] - b[0], height: b[3] - b[1], rotation: page_rotation(doc, page_id) }
}

/// Formats a number for a content stream: short, no exponent.
pub fn fmt(n: f64) -> String {
    if !n.is_finite() {
        return "0".into();
    }
    let s = format!("{:.3}", n);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.to_string()
    }
}

pub fn fmt_matrix(m: &[f64; 6]) -> String {
    m.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: [f64; 6], u: f64, v: f64) -> (f64, f64) {
        (m[0] * u + m[2] * v + m[4], m[1] * u + m[3] * v + m[5])
    }

    /// A user-space point displayed after a clockwise rotation of the page.
    fn displayed(frame: &PageFrame, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = (x - frame.x0, y - frame.y0);
        match frame.rotation {
            90 => (y, frame.width - x),
            180 => (frame.width - x, frame.height - y),
            270 => (frame.height - y, x),
            _ => (x, y),
        }
    }

    #[test]
    fn visual_matrix_round_trips_for_every_rotation() {
        for rotation in [0, 90, 180, 270] {
            let frame = PageFrame { x0: 10.0, y0: 20.0, width: 600.0, height: 800.0, rotation };
            let m = frame.visual_matrix();
            for (u, v) in [(0.0, 0.0), (frame.visual_width(), 0.0), (5.0, frame.visual_height()), (100.0, 37.0)] {
                let (x, y) = apply(m, u, v);
                let (du, dv) = displayed(&frame, x, y);
                assert!((du - u).abs() < 1e-9 && (dv - v).abs() < 1e-9, "rotation {rotation}: ({u},{v}) -> ({du},{dv})");
            }
        }
    }

    #[test]
    fn formats_numbers() {
        assert_eq!(fmt(1.0), "1");
        assert_eq!(fmt(0.5), "0.5");
        assert_eq!(fmt(-0.0001), "0");
        assert_eq!(fmt(12.3456), "12.346");
        assert_eq!(fmt(f64::NAN), "0");
    }
}
