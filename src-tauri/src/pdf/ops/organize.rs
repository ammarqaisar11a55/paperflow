//! Page-level operations: merge, split, organize, rotate and extract pages.

use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension, sanitize_file_name, stem};
use crate::pdf::assemble::{assemble, PageRef};
use crate::pdf::document::{self, normalize_rotation, page_rotation, verify_output};
use crate::pdf::ops::{require_one, Staged};
use crate::pdf::ranges;
use crate::services::context::JobContext;

/// Combines the inputs, in order, into one document.
pub fn merge(ctx: &JobContext, inputs: &[PathBuf], output_name: &str) -> AppResult<Staged> {
    if inputs.len() < 2 {
        return Err(AppError::invalid("Add at least two PDF files to merge."));
    }
    let mut docs = Vec::with_capacity(inputs.len());
    let mut pages = Vec::new();
    let mut warnings = Vec::new();
    for (i, path) in inputs.iter().enumerate() {
        ctx.check()?;
        ctx.progress_file(i as f64 / (inputs.len() + 1) as f64, "Reading documents", display_name(path));
        let doc = document::load(path)?;
        if doc.catalog().map(|c| c.has(b"AcroForm")).unwrap_or(false) && warnings.is_empty() {
            warnings
                .push("Interactive form fields from the merged files may no longer be editable.".to_string());
        }
        let count = doc.get_pages().len() as u32;
        pages.extend((1..=count).map(|p| PageRef::new(i, p)));
        docs.push(doc);
    }
    ctx.progress(inputs.len() as f64 / (inputs.len() + 1) as f64, "Combining pages");
    let mut out = assemble(docs, &pages, false)?;
    ctx.check()?;
    let path = ctx.temp.join("merged.pdf");
    document::save(&mut out, &path)?;
    let count = verify_output(&path, Some(pages.len()))?;
    let mut staged = Staged::single(path, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(count);
    staged.warnings = warnings;
    Ok(staged)
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SplitMode {
    /// One file per comma-separated range.
    #[default]
    Ranges,
    /// One file per page.
    Every,
    /// Files of `chunk_size` pages.
    Chunks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SplitOptions {
    pub mode: SplitMode,
    pub ranges: String,
    pub chunk_size: u32,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self { mode: SplitMode::Ranges, ranges: String::new(), chunk_size: 1 }
    }
}

pub fn split(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: SplitOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    ctx.progress_file(0.0, "Reading document", display_name(input));
    let source = document::load(input)?;
    let total = source.get_pages().len() as u32;
    let fallback = stem(input);
    let base = sanitize_file_name(if output_name.trim().is_empty() { &fallback } else { output_name });
    let base = base.strip_suffix(".pdf").unwrap_or(&base).to_string();
    let width = total.to_string().len();

    let groups: Vec<(Vec<u32>, String)> = match opts.mode {
        SplitMode::Ranges => {
            if opts.ranges.trim().is_empty() {
                return Err(AppError::invalid("Enter the page ranges to split into, for example 1-3, 4-6."));
            }
            ranges::parse_groups(&opts.ranges, total)?
                .into_iter()
                .map(|g| {
                    let label = ranges::describe(&g);
                    (g, format!("{base} (pages {label}).pdf"))
                })
                .collect()
        }
        SplitMode::Every => {
            (1..=total).map(|p| (vec![p], format!("{base} - page {:0width$}.pdf", p))).collect()
        }
        SplitMode::Chunks => ranges::chunks(total, opts.chunk_size)?
            .into_iter()
            .enumerate()
            .map(|(i, g)| {
                (
                    g,
                    format!(
                        "{base} - part {:0w$}.pdf",
                        i + 1,
                        w = (total / opts.chunk_size.max(1) + 1).to_string().len()
                    ),
                )
            })
            .collect(),
    };
    if groups.len() > 2000 {
        return Err(AppError::invalid("That would create more than 2,000 files.")
            .with_hint("Use larger ranges or chunks."));
    }

    let mut staged = Staged { folder: Some(format!("{base} - split")), ..Default::default() };
    let n = groups.len();
    for (i, (pages, name)) in groups.into_iter().enumerate() {
        ctx.check()?;
        ctx.progress(i as f64 / n as f64, format!("Creating file {} of {n}", i + 1));
        let refs: Vec<PageRef> = pages.iter().map(|&p| PageRef::new(0, p)).collect();
        let mut out = assemble(vec![source.clone()], &refs, true)?;
        let path = ctx.temp.join(format!("part-{i}.pdf"));
        document::save(&mut out, &path)?;
        verify_output(&path, Some(refs.len()))?;
        staged.files.push((path, sanitize_file_name(&name)));
    }
    Ok(staged)
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct OrganizedPage {
    /// 1-based page number in the source document.
    pub page: u32,
    /// Extra clockwise rotation in degrees.
    #[serde(default)]
    pub rotate: i64,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OrganizeOptions {
    pub pages: Vec<OrganizedPage>,
}

/// Rebuilds a document from an edited page list (reordered, rotated,
/// deleted and duplicated pages), as produced by the page organizer.
pub fn organize(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: OrganizeOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    if opts.pages.is_empty() {
        return Err(AppError::invalid("The document must keep at least one page."));
    }
    ctx.progress_file(0.1, "Reading document", display_name(input));
    let source = document::load(input)?;
    ctx.check()?;
    ctx.progress(0.5, "Arranging pages");
    let refs: Vec<PageRef> = opts.pages.iter().map(|p| PageRef::rotated(0, p.page, p.rotate)).collect();
    let mut out = assemble(vec![source], &refs, true)?;
    ctx.check()?;
    ctx.progress(0.8, "Saving");
    let path = ctx.temp.join("organized.pdf");
    document::save(&mut out, &path)?;
    let count = verify_output(&path, Some(refs.len()))?;
    let mut staged = Staged::single(path, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(count);
    Ok(staged)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RotateOptions {
    /// Clockwise degrees: 90, 180 or 270 (negative values turn anticlockwise).
    pub angle: i64,
    /// Pages to turn; empty means all.
    pub pages: String,
}

impl Default for RotateOptions {
    fn default() -> Self {
        Self { angle: 90, pages: String::new() }
    }
}

/// Turns pages in place, keeping everything else in the document as it was.
pub fn rotate(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: RotateOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let angle = normalize_rotation(opts.angle);
    if angle == 0 {
        return Err(AppError::invalid("Choose a rotation of 90, 180 or 270 degrees."));
    }
    ctx.progress_file(0.1, "Reading document", display_name(input));
    let mut doc = document::load(input)?;
    let page_ids = doc.get_pages();
    let selected = ranges::parse_pages(&opts.pages, page_ids.len() as u32)?;
    for (i, n) in selected.iter().enumerate() {
        ctx.check()?;
        let id = page_ids[n];
        let r = normalize_rotation(page_rotation(&doc, id) + angle);
        doc.get_dictionary_mut(id)?.set("Rotate", r);
        if i % 50 == 0 {
            ctx.progress(0.1 + 0.7 * i as f64 / selected.len() as f64, "Rotating pages");
        }
    }
    ctx.progress(0.85, "Saving");
    let path = ctx.temp.join("rotated.pdf");
    document::save(&mut doc, &path)?;
    let count = verify_output(&path, Some(page_ids.len()))?;
    let mut staged = Staged::single(path, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(count);
    Ok(staged)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractOptions {
    pub pages: String,
}

/// Copies the selected pages, in the order given, into a new document.
pub fn extract_pages(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: ExtractOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    if opts.pages.trim().is_empty() {
        return Err(AppError::invalid("Enter the pages to extract, for example 1-3, 5."));
    }
    ctx.progress_file(0.1, "Reading document", display_name(input));
    let source = document::load(input)?;
    let pages = ranges::parse_pages(&opts.pages, source.get_pages().len() as u32)?;
    ctx.check()?;
    ctx.progress(0.5, format!("Extracting {} pages", pages.len()));
    let refs: Vec<PageRef> = pages.iter().map(|&p| PageRef::new(0, p)).collect();
    let mut out = assemble(vec![source], &refs, true)?;
    let path = ctx.temp.join("extracted.pdf");
    document::save(&mut out, &path)?;
    let count = verify_output(&path, Some(refs.len()))?;
    let mut staged = Staged::single(path, file_name_with_extension(output_name, "pdf"));
    staged.page_count = Some(count);
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::assemble::tests::{page_texts, sample_doc};
    use lopdf::Document;

    fn write_sample(dir: &std::path::Path, name: &str, pages: u32, label: &str) -> PathBuf {
        let p = dir.join(name);
        sample_doc(pages, label).save(&p).unwrap();
        p
    }

    #[test]
    fn merge_combines_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_sample(dir.path(), "a.pdf", 2, "A");
        let b = write_sample(dir.path(), "b.pdf", 1, "B");
        let ctx = JobContext::for_tests(dir.path());
        let staged = merge(&ctx, &[a.clone(), b], "Combined").unwrap();
        assert_eq!(staged.files[0].1, "Combined.pdf");
        let out = Document::load(&staged.files[0].0).unwrap();
        assert_eq!(page_texts(&out), vec!["A Page 1", "A Page 2", "B Page 1"]);
        assert!(merge(&ctx, &[a], "x").is_err());
    }

    #[test]
    fn split_modes() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_sample(dir.path(), "a.pdf", 5, "A");
        let ctx = JobContext::for_tests(dir.path());

        let s = split(
            &ctx,
            std::slice::from_ref(&a),
            "a",
            SplitOptions { mode: SplitMode::Every, ..Default::default() },
        )
        .unwrap();
        assert_eq!(s.files.len(), 5);
        assert_eq!(s.files[2].1, "a - page 3.pdf");
        assert_eq!(page_texts(&Document::load(&s.files[2].0).unwrap()), vec!["A Page 3"]);

        let s = split(
            &ctx,
            std::slice::from_ref(&a),
            "a",
            SplitOptions { mode: SplitMode::Chunks, chunk_size: 2, ..Default::default() },
        )
        .unwrap();
        assert_eq!(s.files.len(), 3);
        assert_eq!(page_texts(&Document::load(&s.files[2].0).unwrap()), vec!["A Page 5"]);

        let s = split(
            &ctx,
            std::slice::from_ref(&a),
            "a",
            SplitOptions { mode: SplitMode::Ranges, ranges: "1-2, 4-5".into(), ..Default::default() },
        )
        .unwrap();
        assert_eq!(
            s.files.iter().map(|f| f.1.as_str()).collect::<Vec<_>>(),
            vec!["a (pages 1-2).pdf", "a (pages 4-5).pdf"]
        );
        assert_eq!(page_texts(&Document::load(&s.files[1].0).unwrap()), vec!["A Page 4", "A Page 5"]);
        assert!(s.folder.is_some());

        assert!(split(
            &ctx,
            &[a],
            "a",
            SplitOptions { mode: SplitMode::Ranges, ranges: "9".into(), ..Default::default() }
        )
        .is_err());
    }

    #[test]
    fn organize_rotate_extract() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_sample(dir.path(), "a.pdf", 3, "A");
        let ctx = JobContext::for_tests(dir.path());

        let pages = vec![
            OrganizedPage { page: 3, rotate: 90 },
            OrganizedPage { page: 1, rotate: 0 },
            OrganizedPage { page: 3, rotate: 0 },
        ];
        let s = organize(&ctx, std::slice::from_ref(&a), "o", OrganizeOptions { pages }).unwrap();
        let out = Document::load(&s.files[0].0).unwrap();
        assert_eq!(page_texts(&out), vec!["A Page 3", "A Page 1", "A Page 3"]);
        let first = *out.get_pages().get(&1).unwrap();
        assert_eq!(page_rotation(&out, first), 90);

        let s = rotate(&ctx, std::slice::from_ref(&a), "r", RotateOptions { angle: 270, pages: "2".into() })
            .unwrap();
        let out = Document::load(&s.files[0].0).unwrap();
        let ids = out.get_pages();
        assert_eq!(page_rotation(&out, ids[&1]), 0);
        assert_eq!(page_rotation(&out, ids[&2]), 270);
        assert_eq!(page_texts(&out).len(), 3);

        let s = extract_pages(&ctx, std::slice::from_ref(&a), "e", ExtractOptions { pages: "3, 1".into() })
            .unwrap();
        assert_eq!(page_texts(&Document::load(&s.files[0].0).unwrap()), vec!["A Page 3", "A Page 1"]);
        assert!(extract_pages(&ctx, &[a], "e", ExtractOptions { pages: "".into() }).is_err());
    }
}
