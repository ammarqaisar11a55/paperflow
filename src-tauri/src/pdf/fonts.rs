//! The PDF standard fonts used for watermarks and page numbers. They need no
//! embedding: every PDF reader provides them.

use lopdf::{dictionary, Dictionary};
use serde::Deserialize;

use crate::pdf::font_metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum StandardFont {
    #[default]
    Helvetica,
    HelveticaBold,
    TimesRoman,
    TimesBold,
    Courier,
    CourierBold,
}

impl StandardFont {
    pub fn base_font(self) -> &'static str {
        match self {
            StandardFont::Helvetica => "Helvetica",
            StandardFont::HelveticaBold => "Helvetica-Bold",
            StandardFont::TimesRoman => "Times-Roman",
            StandardFont::TimesBold => "Times-Bold",
            StandardFont::Courier => "Courier",
            StandardFont::CourierBold => "Courier-Bold",
        }
    }

    fn widths(self) -> &'static [u16; 224] {
        match self {
            StandardFont::Helvetica => &font_metrics::HELVETICA,
            StandardFont::HelveticaBold => &font_metrics::HELVETICA_BOLD,
            StandardFont::TimesRoman => &font_metrics::TIMES_ROMAN,
            StandardFont::TimesBold => &font_metrics::TIMES_BOLD,
            StandardFont::Courier => &font_metrics::COURIER,
            StandardFont::CourierBold => &font_metrics::COURIER_BOLD,
        }
    }

    /// Cap height as a fraction of the font size, for vertical centring.
    pub fn cap_height(self) -> f64 {
        match self {
            StandardFont::Helvetica | StandardFont::HelveticaBold => 0.718,
            StandardFont::TimesRoman | StandardFont::TimesBold => 0.662,
            StandardFont::Courier | StandardFont::CourierBold => 0.571,
        }
    }

    /// Width in points of WinAnsi-encoded text at `size`.
    pub fn text_width(self, encoded: &[u8], size: f64) -> f64 {
        let widths = self.widths();
        let units: u32 = encoded
            .iter()
            .map(|&b| if b >= 32 { widths[(b - 32) as usize] as u32 } else { 0 })
            .sum();
        units as f64 * size / 1000.0
    }

    pub fn dictionary(self) -> Dictionary {
        dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => self.base_font(),
            "Encoding" => "WinAnsiEncoding",
        }
    }

    fn has_glyph(self, code: u8) -> bool {
        code >= 32 && self.widths()[(code - 32) as usize] > 0
    }
}

/// Encodes text in WinAnsiEncoding. Characters the standard fonts cannot
/// show become "?"; the flag reports whether that happened.
pub fn encode_win_ansi(text: &str, font: StandardFont) -> (Vec<u8>, bool) {
    let mut lossy = false;
    let bytes = text
        .chars()
        .map(|c| {
            let code = match c as u32 {
                0x20..=0x7E | 0xA0..=0xFF => Some(c as u32 as u8),
                _ => match c {
                    '€' => Some(0x80),
                    '‚' => Some(0x82),
                    'ƒ' => Some(0x83),
                    '„' => Some(0x84),
                    '…' => Some(0x85),
                    '†' => Some(0x86),
                    '‡' => Some(0x87),
                    'ˆ' => Some(0x88),
                    '‰' => Some(0x89),
                    'Š' => Some(0x8A),
                    '‹' => Some(0x8B),
                    'Œ' => Some(0x8C),
                    'Ž' => Some(0x8E),
                    '‘' => Some(0x91),
                    '’' => Some(0x92),
                    '“' => Some(0x93),
                    '”' => Some(0x94),
                    '•' => Some(0x95),
                    '–' => Some(0x96),
                    '—' => Some(0x97),
                    '˜' => Some(0x98),
                    '™' => Some(0x99),
                    'š' => Some(0x9A),
                    '›' => Some(0x9B),
                    'œ' => Some(0x9C),
                    'ž' => Some(0x9E),
                    'Ÿ' => Some(0x9F),
                    '\t' | '\n' | '\r' => Some(b' '),
                    _ => None,
                },
            };
            match code {
                Some(b) if font.has_glyph(b) => b,
                _ => {
                    lossy = true;
                    b'?'
                }
            }
        })
        .collect();
    (bytes, lossy)
}

/// A content-stream hex string, which needs no escaping.
pub fn hex_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 2);
    s.push('<');
    for b in bytes {
        s.push_str(&format!("{b:02X}"));
    }
    s.push('>');
    s
}

/// Parses "#RRGGBB" (or "RRGGBB") into PDF colour components 0..1.
pub fn parse_hex_color(input: &str) -> Option<[f64; 3]> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok().map(|n| n as f64 / 255.0);
    Some([v(0)?, v(2)?, v(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_text() {
        // "Hello" in Helvetica: H 722 e 556 l 222 l 222 o 556 = 2278
        let (enc, lossy) = encode_win_ansi("Hello", StandardFont::Helvetica);
        assert!(!lossy);
        assert!((StandardFont::Helvetica.text_width(&enc, 10.0) - 22.78).abs() < 1e-9);
        // Courier is monospaced at 600 units.
        let (enc, _) = encode_win_ansi("abc", StandardFont::Courier);
        assert!((StandardFont::Courier.text_width(&enc, 10.0) - 18.0).abs() < 1e-9);
    }

    #[test]
    fn encodes_win_ansi() {
        assert_eq!(encode_win_ansi("Café €", StandardFont::Helvetica), (vec![b'C', b'a', b'f', 0xE9, b' ', 0x80], false));
        let (enc, lossy) = encode_win_ansi("日本", StandardFont::Helvetica);
        assert_eq!(enc, b"??");
        assert!(lossy);
    }

    #[test]
    fn hex_and_colors() {
        assert_eq!(hex_string(b"A("), "<4128>");
        assert_eq!(parse_hex_color("#ff0000"), Some([1.0, 0.0, 0.0]));
        assert_eq!(parse_hex_color("00FF00"), Some([0.0, 1.0, 0.0]));
        assert_eq!(parse_hex_color("#12"), None);
        assert_eq!(parse_hex_color("#zzzzzz"), None);
    }
}
