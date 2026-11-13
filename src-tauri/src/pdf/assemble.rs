//! Page assembly: builds a new document from pages of one or more source
//! documents, in any order, with optional extra rotation and duplicates.
//!
//! Merge, split, extract and organize are all expressed as an assembly, so
//! the delicate parts (object renumbering, inherited page attributes,
//! duplicated pages, pruning unused objects) live in one tested place.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId};

use crate::error::{AppError, AppResult};
use crate::pdf::document::{inherited, normalize_rotation, page_rotation};

/// One page of the output: page `page` (1-based) of source `source`, turned
/// clockwise by an extra `rotate` degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRef {
    pub source: usize,
    pub page: u32,
    pub rotate: i64,
}

impl PageRef {
    pub fn new(source: usize, page: u32) -> Self {
        Self { source, page, rotate: 0 }
    }

    pub fn rotated(source: usize, page: u32, rotate: i64) -> Self {
        Self { source, page, rotate }
    }
}

/// Document-level entries carried over from the first source when the output
/// is derived from a single document (split, extract, organize).
const KEPT_CATALOG_KEYS: &[&[u8]] = &[
    b"Outlines",
    b"Names",
    b"AcroForm",
    b"Lang",
    b"MarkInfo",
    b"ViewerPreferences",
    b"OCProperties",
    b"PageMode",
];

const INHERITABLE: &[&[u8]] = &[b"Resources", b"MediaBox", b"CropBox"];

fn is_type(obj: &Object, ty: &[u8]) -> bool {
    obj.as_dict().ok().and_then(|d| d.get(b"Type").ok()).and_then(|t| t.as_name().ok()) == Some(ty)
}

/// Builds the output document. With `keep_structure`, bookmarks, named
/// destinations, forms and the document information of the first source are
/// preserved (useful when every page comes from one document).
pub fn assemble(mut sources: Vec<Document>, pages: &[PageRef], keep_structure: bool) -> AppResult<Document> {
    if pages.is_empty() {
        return Err(AppError::invalid("No pages were selected."));
    }

    // 1. Give every source its own range of object numbers.
    let mut next_id = 1;
    let mut page_maps: Vec<BTreeMap<u32, ObjectId>> = Vec::with_capacity(sources.len());
    for doc in sources.iter_mut() {
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;
        page_maps.push(doc.get_pages());
    }

    for p in pages {
        let count = page_maps.get(p.source).map(|m| m.len()).unwrap_or(0);
        if !page_maps.get(p.source).is_some_and(|m| m.contains_key(&p.page)) {
            return Err(AppError::invalid(format!(
                "Page {} does not exist (the document has {count} pages).",
                p.page
            )));
        }
    }

    // 2. Copy inherited attributes onto each used page, since the old page
    //    trees are dropped, and remember each page's own rotation.
    let mut base_rotation: HashMap<ObjectId, i64> = HashMap::new();
    let mut used: Vec<BTreeSet<ObjectId>> = vec![BTreeSet::new(); sources.len()];
    for p in pages {
        used[p.source].insert(page_maps[p.source][&p.page]);
    }
    for (i, doc) in sources.iter_mut().enumerate() {
        for &id in &used[i] {
            base_rotation.insert(id, page_rotation(doc, id));
            let mut additions = Vec::new();
            for key in INHERITABLE {
                let has = doc.get_dictionary(id).map(|d| d.has(key)).unwrap_or(false);
                if !has {
                    if let Some(v) = inherited(doc, id, key) {
                        additions.push((key.to_vec(), v));
                    }
                }
            }
            let dict = doc.get_dictionary_mut(id)?;
            for (k, v) in additions {
                dict.set(k, v);
            }
            if !dict.has(b"MediaBox") {
                dict.set("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()]);
            }
            if !dict.has(b"Resources") {
                dict.set("Resources", Dictionary::new());
            }
        }
    }

    // 3. Document-level entries to keep.
    let version = sources
        .iter()
        .map(|d| d.version.clone())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or_else(|| "1.7".into());
    let mut catalog = Dictionary::new();
    let mut info: Option<Object> = None;
    if keep_structure {
        if let Some(first) = sources.first() {
            if let Ok(src) = first.catalog() {
                for key in KEPT_CATALOG_KEYS {
                    if let Ok(v) = src.get(key) {
                        catalog.set(key.to_vec(), v.clone());
                    }
                }
            }
            info = first.trailer.get(b"Info").ok().cloned();
        }
    }

    // 4. Move objects into the output, leaving behind catalogs, page tree
    //    nodes and pages that are not used.
    let mut out = Document::with_version(if version.is_empty() { "1.7".to_string() } else { version });
    for (i, doc) in sources.into_iter().enumerate() {
        let root = doc.trailer.get(b"Root").and_then(Object::as_reference).ok();
        for (id, obj) in doc.objects {
            if Some(id) == root
                || is_type(&obj, b"Pages")
                || (is_type(&obj, b"Page") && !used[i].contains(&id))
            {
                continue;
            }
            out.objects.insert(id, obj);
        }
    }
    out.max_id = next_id.saturating_sub(1).max(out.objects.keys().map(|k| k.0).max().unwrap_or(0));

    // 5. Build the new page tree.
    let pages_id = out.new_object_id();
    let mut kids = Vec::with_capacity(pages.len());
    let mut placed: HashSet<ObjectId> = HashSet::new();
    for p in pages {
        let original = page_maps[p.source][&p.page];
        let id = if placed.insert(original) { original } else { duplicate_page(&mut out, original)? };
        let rotation = normalize_rotation(base_rotation.get(&original).copied().unwrap_or(0) + p.rotate);
        let dict = out.get_dictionary_mut(id)?;
        dict.set("Parent", pages_id);
        if rotation == 0 {
            dict.remove(b"Rotate");
        } else {
            dict.set("Rotate", rotation);
        }
        kids.push(Object::Reference(id));
    }
    let count = kids.len() as i64;
    out.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );

    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", pages_id);
    let catalog_id = out.add_object(catalog);
    out.trailer.set("Root", catalog_id);
    if let Some(info) = info {
        out.trailer.set("Info", info);
    }

    out.prune_objects();
    out.renumber_objects();
    Ok(out)
}

/// Copies a page object (and its annotations, which belong to exactly one
/// page) so the same page can appear twice.
fn duplicate_page(doc: &mut Document, original: ObjectId) -> AppResult<ObjectId> {
    let mut dict = doc.get_dictionary(original)?.clone();
    let annots: Vec<Object> = match dict.get(b"Annots") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(Object::Reference(r)) => {
            doc.get_object(*r).and_then(Object::as_array).cloned().unwrap_or_default()
        }
        _ => Vec::new(),
    };
    dict.remove(b"Annots");
    let new_id = doc.add_object(dict);
    if !annots.is_empty() {
        let mut copies = Vec::with_capacity(annots.len());
        for a in annots {
            let annot = match &a {
                Object::Reference(r) => doc.get_object(*r).ok().cloned(),
                other => Some(other.clone()),
            };
            if let Some(Object::Dictionary(mut d)) = annot {
                d.set("P", new_id);
                d.remove(b"Popup");
                copies.push(Object::Reference(doc.add_object(d)));
            }
        }
        doc.get_dictionary_mut(new_id)?.set("Annots", copies);
    }
    Ok(new_id)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::Stream;

    /// Builds a document whose page `n` shows the text "Page n" and whose
    /// MediaBox and Resources live on the page tree node (inherited).
    pub fn sample_doc(pages: u32, label: &str) -> Document {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
        let mut kids = Vec::new();
        for n in 1..=pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![72.into(), 700.into()]),
                    Operation::new("Tj", vec![Object::string_literal(format!("{label} Page {n}"))]),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            });
            kids.push(page_id.into());
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => kids, "Count" => pages as i64,
                "Resources" => resources_id, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    pub fn page_texts(doc: &Document) -> Vec<String> {
        doc.get_pages()
            .keys()
            .map(|n| doc.extract_text(&[*n]).unwrap_or_default().trim().to_string())
            .collect()
    }

    fn roundtrip(mut doc: Document) -> Document {
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        Document::load_mem(&buf).unwrap()
    }

    #[test]
    fn merges_in_order() {
        let a = sample_doc(2, "A");
        let b = sample_doc(3, "B");
        let pages = vec![
            PageRef::new(0, 1),
            PageRef::new(0, 2),
            PageRef::new(1, 1),
            PageRef::new(1, 2),
            PageRef::new(1, 3),
        ];
        let out = roundtrip(assemble(vec![a, b], &pages, false).unwrap());
        assert_eq!(page_texts(&out), vec!["A Page 1", "A Page 2", "B Page 1", "B Page 2", "B Page 3"]);
        // Inherited attributes were copied onto the pages.
        for id in out.get_pages().values() {
            let d = out.get_dictionary(*id).unwrap();
            assert!(d.has(b"MediaBox"));
            assert!(d.has(b"Resources"));
        }
    }

    #[test]
    fn reorders_rotates_and_duplicates() {
        let a = sample_doc(3, "A");
        let pages = vec![PageRef::rotated(0, 3, 90), PageRef::new(0, 1), PageRef::rotated(0, 1, -90)];
        let out = roundtrip(assemble(vec![a], &pages, true).unwrap());
        assert_eq!(page_texts(&out), vec!["A Page 3", "A Page 1", "A Page 1"]);
        let ids: Vec<_> = out.get_pages().values().copied().collect();
        assert_eq!(page_rotation(&out, ids[0]), 90);
        assert_eq!(page_rotation(&out, ids[1]), 0);
        assert_eq!(page_rotation(&out, ids[2]), 270);
        assert_ne!(ids[1], ids[2]);
    }

    #[test]
    fn drops_unused_pages() {
        let a = sample_doc(10, "A");
        let mut full = a.clone();
        let mut full_bytes = Vec::new();
        full.save_to(&mut full_bytes).unwrap();
        let out = assemble(vec![a], &[PageRef::new(0, 4)], true).unwrap();
        let mut out = roundtrip(out);
        assert_eq!(page_texts(&out), vec!["A Page 4"]);
        let mut bytes = Vec::new();
        out.save_to(&mut bytes).unwrap();
        assert!(bytes.len() < full_bytes.len() / 2);
    }

    #[test]
    fn rejects_missing_pages() {
        assert!(assemble(vec![sample_doc(2, "A")], &[PageRef::new(0, 3)], false).is_err());
        assert!(assemble(vec![sample_doc(2, "A")], &[PageRef::new(1, 1)], false).is_err());
        assert!(assemble(vec![sample_doc(2, "A")], &[], false).is_err());
    }
}
