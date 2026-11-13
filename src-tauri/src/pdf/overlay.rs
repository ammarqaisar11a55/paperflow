//! Drawing on top of existing pages without disturbing their content.
//!
//! The page's original content is wrapped in `q ... Q` so whatever graphics
//! state it leaves behind cannot affect the overlay, and the overlay's
//! resources are added under names that cannot clash with existing ones.

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::error::AppResult;
use crate::pdf::document::inherited;

#[derive(Debug, Default)]
pub struct Overlay {
    /// Content-stream operators drawn after the page content.
    pub content: Vec<u8>,
    pub fonts: Vec<(String, Object)>,
    pub ext_gstates: Vec<(String, Object)>,
    pub xobjects: Vec<(String, Object)>,
}

fn resolve_dict(doc: &Document, obj: Option<&Object>) -> Dictionary {
    match obj {
        Some(Object::Dictionary(d)) => d.clone(),
        Some(Object::Reference(id)) => doc.get_dictionary(*id).cloned().unwrap_or_default(),
        _ => Dictionary::new(),
    }
}

fn merge_category(doc: &Document, resources: &mut Dictionary, key: &[u8], entries: &[(String, Object)]) {
    if entries.is_empty() {
        return;
    }
    let mut cat = resolve_dict(doc, resources.get(key).ok());
    for (name, value) in entries {
        cat.set(name.as_bytes().to_vec(), value.clone());
    }
    resources.set(key.to_vec(), Object::Dictionary(cat));
}

/// Adds an overlay to one page.
pub fn apply(doc: &mut Document, page_id: ObjectId, overlay: &Overlay) -> AppResult<()> {
    // Resources: copy the effective dictionary onto the page so shared or
    // inherited dictionaries used by other pages stay untouched.
    let effective = inherited(doc, page_id, b"Resources");
    let mut resources = resolve_dict(doc, effective.as_ref());
    merge_category(doc, &mut resources, b"Font", &overlay.fonts);
    merge_category(doc, &mut resources, b"ExtGState", &overlay.ext_gstates);
    merge_category(doc, &mut resources, b"XObject", &overlay.xobjects);
    if !overlay.fonts.is_empty() {
        // Text needs the Text procedure set on very old readers.
        if !resources.has(b"ProcSet") {
            resources.set("ProcSet", vec![Object::Name(b"PDF".to_vec()), Object::Name(b"Text".to_vec())]);
        }
    }

    // Contents: [q] + original + [Q overlay].
    let existing: Vec<Object> = match doc.get_dictionary(page_id)?.get(b"Contents") {
        Ok(Object::Reference(id)) => match doc.get_object(*id) {
            Ok(Object::Array(a)) => a.clone(),
            _ => vec![Object::Reference(*id)],
        },
        Ok(Object::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    let open = doc.add_object(Stream::new(Dictionary::new(), b"q\n".to_vec()));
    let mut body = b"\nQ\n".to_vec();
    body.extend_from_slice(&overlay.content);
    let close = doc.add_object(Stream::new(Dictionary::new(), body));

    let mut contents = Vec::with_capacity(existing.len() + 2);
    contents.push(Object::Reference(open));
    contents.extend(existing);
    contents.push(Object::Reference(close));

    let page = doc.get_dictionary_mut(page_id)?;
    page.set("Resources", Object::Dictionary(resources));
    page.set("Contents", contents);
    Ok(())
}
