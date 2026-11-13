//! Loading and saving documents with friendly errors.

use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use crate::error::{AppError, AppResult, ErrorKind};
use crate::filesystem::paths::{display_name, validate_pdf_input};

/// Loads a PDF for editing. Encrypted documents are refused: PaperFlow only
/// works on protected files after the user unlocks them with their password.
pub fn load(path: &Path) -> AppResult<Document> {
    validate_pdf_input(path)?;
    let name = display_name(path);
    let doc = Document::load(path).map_err(|e| match e {
        lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => AppError::encrypted(&name),
        other => AppError::invalid(format!("“{name}” could not be read. It may be damaged."))
            .with_hint("Try opening and re-saving it in another PDF viewer, then try again.")
            .with_detail(other.to_string()),
    })?;
    if doc.is_encrypted() || doc.was_encrypted() {
        // Opens without a password but carries an owner password with
        // usage restrictions. Lifting those needs the owner password.
        return Err(AppError::new(
            ErrorKind::Encrypted,
            format!("“{name}” has restrictions set by its author."),
        )
        .with_hint("Remove them with Unlock PDF and the document's owner password, then try again."));
    }
    if doc.get_pages().is_empty() {
        return Err(AppError::invalid(format!("“{name}” has no pages.")));
    }
    Ok(doc)
}

/// Saves a document, compressing any uncompressed streams first.
pub fn save(doc: &mut Document, path: &Path) -> AppResult<()> {
    doc.compress();
    doc.save(path)
        .map_err(|e| AppError::output("The result could not be written.").with_detail(e.to_string()))?;
    Ok(())
}

/// Opens a finished output again to make sure it is a readable PDF with the
/// expected number of pages.
pub fn verify_output(path: &Path, expected_pages: Option<usize>) -> AppResult<usize> {
    let doc = Document::load(path).map_err(|e| {
        AppError::processing("The result failed validation and was discarded.").with_detail(e.to_string())
    })?;
    let count = doc.get_pages().len();
    if count == 0 {
        return Err(AppError::processing("The result has no pages and was discarded."));
    }
    if let Some(expected) = expected_pages {
        if count != expected {
            return Err(AppError::processing("The result failed validation and was discarded.")
                .with_detail(format!("expected {expected} pages, found {count}")));
        }
    }
    Ok(count)
}

/// Looks up a page attribute, following the page tree upwards for the
/// inheritable ones (Resources, MediaBox, CropBox, Rotate).
pub fn inherited(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut dict = doc.get_dictionary(page_id).ok()?;
    for _ in 0..64 {
        if let Ok(v) = dict.get(key) {
            return Some(v.clone());
        }
        let parent = dict.get(b"Parent").and_then(Object::as_reference).ok()?;
        dict = doc.get_dictionary(parent).ok()?;
    }
    None
}

/// Resolves a possibly indirect object to a number.
pub fn number(doc: &Document, obj: &Object) -> Option<f64> {
    let obj = match obj {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        o => o,
    };
    match obj {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

/// The page's effective rotation, normalised to 0, 90, 180 or 270.
pub fn page_rotation(doc: &Document, page_id: ObjectId) -> i64 {
    inherited(doc, page_id, b"Rotate")
        .and_then(|o| number(doc, &o))
        .map(|r| normalize_rotation(r as i64))
        .unwrap_or(0)
}

pub fn normalize_rotation(degrees: i64) -> i64 {
    let r = degrees.rem_euclid(360);
    // Rotate must be a multiple of 90; round anything odd to the nearest.
    ((r + 45) / 90 % 4) * 90
}

/// Plain-text value of an Info dictionary entry.
pub fn info_string(doc: &Document, key: &[u8]) -> Option<String> {
    let info = doc.trailer.get(b"Info").ok()?;
    let dict = match info {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    let value = dict.get(key).ok()?;
    let value = match value {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        v => v,
    };
    lopdf::decode_text_string(value).ok().map(|s| s.trim_end_matches('\0').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_rotation() {
        assert_eq!(normalize_rotation(0), 0);
        assert_eq!(normalize_rotation(90), 90);
        assert_eq!(normalize_rotation(-90), 270);
        assert_eq!(normalize_rotation(450), 90);
        assert_eq!(normalize_rotation(360), 0);
        assert_eq!(normalize_rotation(-180), 180);
        assert_eq!(normalize_rotation(100), 90);
    }
}
