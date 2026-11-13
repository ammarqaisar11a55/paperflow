//! Reading and editing the document information dictionary.

use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::filesystem::paths::{display_name, file_name_with_extension, validate_pdf_input};
use crate::pdf::document::{self, info_string, verify_output};
use crate::pdf::ops::{require_one, Staged};
use crate::services::context::JobContext;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
}

/// Reads one field of the metadata.
type Field = fn(&Metadata) -> &str;

const FIELDS: [(&[u8], Field); 6] = [
    (b"Title", |m| &m.title),
    (b"Author", |m| &m.author),
    (b"Subject", |m| &m.subject),
    (b"Keywords", |m| &m.keywords),
    (b"Creator", |m| &m.creator),
    (b"Producer", |m| &m.producer),
];

/// Reads the current metadata so the editor can show it.
pub fn read(path: &Path) -> AppResult<Metadata> {
    validate_pdf_input(path)?;
    let doc = document::load(path)?;
    let get = |k: &[u8]| info_string(&doc, k).unwrap_or_default();
    Ok(Metadata {
        title: get(b"Title"),
        author: get(b"Author"),
        subject: get(b"Subject"),
        keywords: get(b"Keywords"),
        creator: get(b"Creator"),
        producer: get(b"Producer"),
    })
}

/// PDF date string for "now", e.g. D:20261113091500+05'00'.
fn pdf_date_now() -> String {
    let now = chrono::Local::now();
    let offset = now.format("%z").to_string(); // +0500
    let (sign_h, m) = offset.split_at(3);
    format!("D:{}{}'{}'", now.format("%Y%m%d%H%M%S"), sign_h, m)
}

pub fn write(ctx: &JobContext, inputs: &[PathBuf], output_name: &str, meta: Metadata) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    ctx.progress_file(0.1, "Reading document", display_name(input));
    let mut doc = document::load(input)?;
    ctx.check()?;

    let mut info: Dictionary = match doc.trailer.get(b"Info") {
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).cloned().unwrap_or_default(),
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    for (key, get) in FIELDS {
        let value = get(&meta).trim();
        if value.is_empty() {
            info.remove(key);
        } else {
            info.set(key.to_vec(), lopdf::text_string(value));
        }
    }
    info.set("ModDate", Object::string_literal(pdf_date_now()));
    let info_id = doc.add_object(info);
    doc.trailer.set("Info", info_id);
    // An XMP packet would contradict the edited values in readers that
    // prefer it, so the document information dictionary becomes the only
    // source of metadata.
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"Metadata");
    }

    ctx.progress(0.7, "Saving");
    let path = ctx.temp.join("metadata.pdf");
    let pages = doc.get_pages().len();
    document::save(&mut doc, &path)?;
    let mut staged = Staged::single(path.clone(), file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(verify_output(&path, Some(pages))?);
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::assemble::tests::sample_doc;

    #[test]
    fn writes_and_reads_unicode_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        sample_doc(1, "A").save(&input).unwrap();
        let ctx = JobContext::for_tests(dir.path());
        let meta = Metadata {
            title: "Quarterly Report — Été".into(),
            author: "Ada Lovelace".into(),
            keywords: "pdf, offline".into(),
            ..Default::default()
        };
        let s = write(&ctx, &[input], "m", meta.clone()).unwrap();
        let back = read(&s.files[0].0).unwrap();
        assert_eq!(back.title, meta.title);
        assert_eq!(back.author, meta.author);
        assert_eq!(back.keywords, meta.keywords);
        assert_eq!(back.subject, "");
    }

    #[test]
    fn date_format() {
        let d = pdf_date_now();
        assert!(d.starts_with("D:") && d.ends_with('\''));
        assert_eq!(d.len(), 23);
    }
}
