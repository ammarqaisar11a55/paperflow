//! PDF to Markdown.
//!
//! Poppler's `pdftohtml -xml` reports every run of text with its position and
//! font. From that, PaperFlow rebuilds the document's structure:
//!
//! - lines are rebuilt from runs that share a baseline;
//! - headings come from text noticeably larger than the body text, ranked by
//!   size;
//! - consecutive lines merge into paragraphs (with hyphenated words rejoined)
//!   until a larger vertical gap;
//! - bullet and numbered lines become list items, and their wrapped lines are
//!   joined to them;
//! - bold, italic and links are kept;
//! - headers and footers repeated on most pages (running titles, page
//!   numbers) are dropped.
//!
//! Tables and multi-column layouts come out as text in reading order. Scanned
//! pages have no text; OCR them first.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, file_name_with_extension};
use crate::pdf::document;
use crate::pdf::ops::{require_one, Staged};
use crate::pdf::ranges;
use crate::services::context::JobContext;
use crate::services::engines::{self, Engine};

// ------------------------------------------------------------------ model

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub link: Option<String>,
}

/// One run of text as positioned by pdftohtml.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Run {
    pub top: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
    pub font: Font,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub number: u32,
    pub height: f64,
    pub runs: Vec<Run>,
}

// ------------------------------------------------------------------ parsing

fn attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

fn attr_f64(e: &BytesStart, name: &[u8]) -> f64 {
    attr(e, name).and_then(|v| v.parse().ok()).unwrap_or(0.0)
}

/// Parses `pdftohtml -xml` output.
pub fn parse_xml(xml: &str) -> AppResult<Vec<Page>> {
    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.trim_text(false);
    config.check_end_names = false;

    let mut fonts: HashMap<String, Font> = HashMap::new();
    let mut pages: Vec<Page> = Vec::new();
    let mut run: Option<Run> = None;
    let (mut bold, mut italic) = (0u32, 0u32);
    let mut link: Option<String> = None;

    let bad = |e: String| AppError::processing("The document layout could not be read.").with_detail(e);
    loop {
        match reader.read_event().map_err(|e| bad(e.to_string()))? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"page" => {
                pages.push(Page {
                    number: attr(&e, b"number")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(pages.len() as u32 + 1),
                    height: attr_f64(&e, b"height"),
                    runs: Vec::new(),
                });
            }
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"fontspec" => {
                let family = attr(&e, b"family").unwrap_or_default().to_lowercase();
                fonts.insert(
                    attr(&e, b"id").unwrap_or_default(),
                    Font {
                        size: attr_f64(&e, b"size"),
                        bold: family.contains("bold") || family.contains("black") || family.contains("heavy"),
                        italic: family.contains("italic") || family.contains("oblique"),
                    },
                );
            }
            Event::Start(e) if e.name().as_ref() == b"text" => {
                let font = fonts.get(&attr(&e, b"font").unwrap_or_default()).cloned().unwrap_or_default();
                run = Some(Run {
                    top: attr_f64(&e, b"top"),
                    left: attr_f64(&e, b"left"),
                    width: attr_f64(&e, b"width"),
                    height: attr_f64(&e, b"height"),
                    font,
                    spans: Vec::new(),
                });
                bold = 0;
                italic = 0;
                link = None;
            }
            Event::Start(e) if run.is_some() => match e.name().as_ref() {
                b"b" => bold += 1,
                b"i" => italic += 1,
                b"a" => link = attr(&e, b"href"),
                _ => {}
            },
            Event::End(e) => match e.name().as_ref() {
                b"text" => {
                    if let (Some(r), Some(page)) = (run.take(), pages.last_mut()) {
                        if r.spans.iter().any(|s| !s.text.trim().is_empty()) {
                            page.runs.push(r);
                        }
                    }
                }
                b"b" => bold = bold.saturating_sub(1),
                b"i" => italic = italic.saturating_sub(1),
                b"a" => link = None,
                _ => {}
            },
            Event::Text(t) => {
                if let Some(r) = run.as_mut() {
                    let text = t.decode().map_err(|e| bad(e.to_string()))?;
                    push_text(r, &text, bold > 0, italic > 0, &link);
                }
            }
            // Entities (`&amp;`, `&#160;`) arrive as separate events.
            Event::GeneralRef(entity) => {
                if let Some(r) = run.as_mut() {
                    let resolved = match entity.resolve_char_ref() {
                        Ok(Some(c)) => c.to_string(),
                        _ => {
                            let name = entity.decode().map_err(|e| bad(e.to_string()))?;
                            match name.as_ref() {
                                "amp" => "&",
                                "lt" => "<",
                                "gt" => ">",
                                "quot" => "\"",
                                "apos" => "'",
                                "nbsp" => " ",
                                _ => "",
                            }
                            .to_string()
                        }
                    };
                    push_text(r, &resolved, bold > 0, italic > 0, &link);
                }
            }
            _ => {}
        }
    }
    Ok(pages)
}

/// Adds text to a run, extending the previous span when the style matches.
fn push_text(run: &mut Run, text: &str, bold: bool, italic: bool, link: &Option<String>) {
    let text = text.replace('\u{a0}', " ");
    if text.is_empty() {
        return;
    }
    let (bold, italic) = (bold || run.font.bold, italic || run.font.italic);
    match run.spans.last_mut() {
        Some(s) if s.bold == bold && s.italic == italic && &s.link == link => s.text.push_str(&text),
        _ => run.spans.push(Span { text, bold, italic, link: link.clone() }),
    }
}

// ------------------------------------------------------------------ layout

#[derive(Debug, Clone)]
struct Line {
    top: f64,
    left: f64,
    height: f64,
    size: f64,
    spans: Vec<Span>,
}

impl Line {
    fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect::<String>().trim().to_string()
    }
}

fn round_size(s: f64) -> i64 {
    (s * 2.0).round() as i64
}

/// Groups a page's runs into lines (runs sharing a baseline), left to right.
fn lines_of(page: &Page) -> Vec<Line> {
    let mut runs = page.runs.clone();
    runs.sort_by(|a, b| a.top.total_cmp(&b.top).then(a.left.total_cmp(&b.left)));
    let mut lines: Vec<(Line, Vec<Run>)> = Vec::new();
    for r in runs {
        let tolerance = (r.height * 0.4).max(2.0);
        match lines.last_mut() {
            Some((line, members)) if (r.top - line.top).abs() <= tolerance => {
                line.height = line.height.max(r.height);
                members.push(r);
            }
            _ => {
                let line = Line { top: r.top, left: r.left, height: r.height, size: 0.0, spans: Vec::new() };
                lines.push((line, vec![r]));
            }
        }
    }
    lines
        .into_iter()
        .map(|(mut line, mut members)| {
            members.sort_by(|a, b| a.left.total_cmp(&b.left));
            line.left = members[0].left;
            // The line's size is the size most of its characters use.
            let mut weights: HashMap<i64, usize> = HashMap::new();
            let mut right: Option<f64> = None;
            for r in &members {
                let chars: usize = r.spans.iter().map(|s| s.text.chars().count()).sum();
                *weights.entry(round_size(r.font.size)).or_default() += chars;
                if let Some(prev_right) = right {
                    let gap = r.left - prev_right;
                    let ends_with_space = line.spans.last().is_some_and(|s| s.text.ends_with(' '));
                    if gap > r.height * 0.15 && !ends_with_space {
                        line.spans.push(Span { text: " ".into(), ..Default::default() });
                    }
                }
                right = Some(r.left + r.width);
                line.spans.extend(r.spans.iter().cloned());
            }
            line.size =
                weights.into_iter().max_by_key(|(_, n)| *n).map(|(s, _)| s as f64 / 2.0).unwrap_or(0.0);
            line
        })
        .filter(|l| !l.text().is_empty())
        .collect()
}

/// Text with digits folded, to recognise "Page 3" and "Page 4" as the same.
fn signature(text: &str) -> String {
    text.chars().map(|c| if c.is_ascii_digit() { '#' } else { c.to_ascii_lowercase() }).collect::<String>()
}

/// Removes running headers and footers: lines near the top or bottom edge
/// whose text (ignoring numbers) repeats on most pages.
fn strip_running_lines(pages: &mut [(u32, f64, Vec<Line>)]) {
    if pages.len() < 3 {
        return;
    }
    let edge =
        |height: f64, l: &Line| height > 0.0 && (l.top < height * 0.08 || l.top + l.height > height * 0.92);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_, height, lines) in pages.iter() {
        let mut seen = std::collections::HashSet::new();
        for l in lines.iter().filter(|l| edge(*height, l)) {
            let sig = signature(&l.text());
            if seen.insert(sig.clone()) {
                *counts.entry(sig).or_default() += 1;
            }
        }
    }
    let threshold = (pages.len() as f64 * 0.5).ceil().max(2.0) as usize;
    for (_, height, lines) in pages.iter_mut() {
        let h = *height;
        lines.retain(|l| {
            !(edge(h, l) && counts.get(&signature(&l.text())).copied().unwrap_or(0) >= threshold)
        });
    }
}

// ------------------------------------------------------------------ blocks

#[derive(Debug, Clone, PartialEq)]
enum Block {
    Heading(usize, Vec<Span>),
    Paragraph(Vec<Span>),
    Item { marker: Option<String>, spans: Vec<Span>, left: f64 },
    PageBreak,
}

const BULLETS: &[char] =
    &['•', '◦', '▪', '▫', '‣', '∙', '·', '●', '○', '■', '□', '►', '▸', '✓', '–', '-', '*'];

/// If a line starts like a list item, returns (ordered marker, rest).
fn list_marker(text: &str) -> Option<(Option<String>, String)> {
    let t = text.trim_start();
    let first = t.chars().next()?;
    if BULLETS.contains(&first) {
        let rest = &t[first.len_utf8()..];
        // A dash or star needs a space after it to be a bullet.
        if matches!(first, '-' | '*' | '–') && !rest.starts_with(' ') {
            return None;
        }
        return Some((None, rest.trim_start().to_string()));
    }
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() && digits.len() <= 3 {
        let after = &t[digits.len()..];
        if (after.starts_with(". ") || after.starts_with(") ")) && after.len() > 2 {
            return Some((Some(format!("{digits}.")), after[2..].trim_start().to_string()));
        }
    }
    None
}

/// Removes the list marker from a line's spans, keeping `text_after` (a
/// suffix of the line's trimmed text) and its styling.
fn drop_prefix(spans: &[Span], text_after: &str) -> Vec<Span> {
    let full: String = spans.iter().map(|s| s.text.as_str()).collect();
    let leading = full.len() - full.trim_start().len();
    let skip = leading + full.trim().len().saturating_sub(text_after.len());
    let mut remaining = skip;
    let mut out = Vec::new();
    for s in spans {
        if remaining >= s.text.len() {
            remaining -= s.text.len();
            continue;
        }
        let mut s = s.clone();
        s.text = s.text[remaining..].to_string();
        remaining = 0;
        out.push(s);
    }
    out
}

/// Appends a line to a block's spans, joining hyphenated words.
fn append(spans: &mut Vec<Span>, next: &[Span]) {
    let prev_text: String = spans.iter().map(|s| s.text.as_str()).collect();
    let next_text: String = next.iter().map(|s| s.text.as_str()).collect();
    let trimmed = prev_text.trim_end();
    let hyphenated = trimmed.ends_with('-')
        && trimmed.chars().rev().nth(1).is_some_and(|c| c.is_alphabetic())
        && next_text.trim_start().chars().next().is_some_and(|c| c.is_lowercase());
    if hyphenated {
        // Remove the trailing hyphen (and any trailing spaces) from the last spans.
        while let Some(last) = spans.last_mut() {
            let t = last.text.trim_end().to_string();
            if t.is_empty() {
                spans.pop();
                continue;
            }
            last.text = t.strip_suffix('-').unwrap_or(&t).to_string();
            break;
        }
    } else if !prev_text.ends_with(' ') && !next_text.starts_with(' ') {
        spans.push(Span { text: " ".into(), ..Default::default() });
    }
    spans.extend(next.iter().cloned());
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MarkdownOptions {
    pub pages: String,
    /// Turn larger text into headings.
    pub detect_headings: bool,
    /// Put a horizontal rule between pages.
    pub page_breaks: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self { pages: String::new(), detect_headings: true, page_breaks: false }
    }
}

/// Builds Markdown from parsed pages.
pub fn to_markdown(pages: &[Page], opts: &MarkdownOptions) -> String {
    let mut per_page: Vec<(u32, f64, Vec<Line>)> =
        pages.iter().map(|p| (p.number, p.height, lines_of(p))).collect();
    strip_running_lines(&mut per_page);

    // The body size is the one most characters are set in.
    let mut weights: BTreeMap<i64, usize> = BTreeMap::new();
    for (_, _, lines) in &per_page {
        for l in lines {
            *weights.entry(round_size(l.size)).or_default() += l.text().chars().count();
        }
    }
    let body = weights.iter().max_by_key(|(_, n)| **n).map(|(s, _)| *s as f64 / 2.0).unwrap_or(0.0);
    let mut heading_sizes: Vec<i64> =
        weights.keys().copied().filter(|s| body > 0.0 && (*s as f64 / 2.0) >= body * 1.18).collect();
    heading_sizes.sort_unstable_by(|a, b| b.cmp(a));
    let level_of = |size: f64| -> Option<usize> {
        if !opts.detect_headings {
            return None;
        }
        heading_sizes.iter().position(|s| *s == round_size(size)).map(|i| (i + 1).min(4))
    };

    let mut blocks: Vec<Block> = Vec::new();
    for (index, (_, _, lines)) in per_page.iter().enumerate() {
        if index > 0 && opts.page_breaks && !blocks.is_empty() {
            blocks.push(Block::PageBreak);
        }
        let mut prev: Option<&Line> = None;
        for line in lines {
            let text = line.text();
            let gap = prev.map(|p| line.top - (p.top + p.height)).unwrap_or(f64::MAX);
            let close = prev.is_some() && gap < line.height.max(1.0) * 0.8;
            let level = if text.chars().count() <= 200 { level_of(line.size) } else { None };

            if let Some(level) = level {
                match blocks.last_mut() {
                    Some(Block::Heading(l, spans)) if *l == level && close => append(spans, &line.spans),
                    _ => blocks.push(Block::Heading(level, line.spans.clone())),
                }
            } else if let Some((marker, rest)) = list_marker(&text) {
                blocks.push(Block::Item { marker, spans: drop_prefix(&line.spans, &rest), left: line.left });
            } else {
                match blocks.last_mut() {
                    // Wrapped line of a list item: indented past the marker.
                    Some(Block::Item { spans, left, .. }) if close && line.left > *left + 2.0 => {
                        append(spans, &line.spans)
                    }
                    Some(Block::Paragraph(spans)) if close => append(spans, &line.spans),
                    _ => blocks.push(Block::Paragraph(line.spans.clone())),
                }
            }
            prev = Some(line);
        }
    }
    render(&blocks)
}

// ------------------------------------------------------------------ output

/// Escapes characters Markdown would otherwise interpret.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Spans to inline Markdown: emphasis and links, with neighbouring spans of
/// the same style merged.
fn inline(spans: &[Span], plain: bool) -> String {
    let mut merged: Vec<Span> = Vec::new();
    for s in spans {
        match merged.last_mut() {
            Some(m) if m.bold == s.bold && m.italic == s.italic && m.link == s.link => {
                m.text.push_str(&s.text)
            }
            _ => merged.push(s.clone()),
        }
    }
    let mut out = String::new();
    for s in merged {
        let core = s.text.trim();
        if core.is_empty() {
            out.push_str(if s.text.is_empty() { "" } else { " " });
            continue;
        }
        let lead = &s.text[..s.text.len() - s.text.trim_start().len()];
        let trail = &s.text[s.text.trim_end().len()..];
        let mut piece = escape(core);
        if let Some(url) = &s.link {
            piece = if core == url { format!("<{url}>") } else { format!("[{piece}]({url})") };
        }
        if !plain {
            let marks = match (s.bold, s.italic) {
                (true, true) => "***",
                (true, false) => "**",
                (false, true) => "*",
                _ => "",
            };
            piece = format!("{marks}{piece}{marks}");
        }
        out.push_str(lead);
        out.push_str(&piece);
        out.push_str(trail);
    }
    // Collapse runs of spaces left by joined lines.
    let mut collapsed = String::with_capacity(out.len());
    for c in out.trim().chars() {
        if c == ' ' && collapsed.ends_with(' ') {
            continue;
        }
        collapsed.push(c);
    }
    collapsed
}

/// Keeps a paragraph from being read as a heading, list or rule.
fn guard_block_start(text: String) -> String {
    if text.starts_with('#') || text.starts_with('+') || text.starts_with("- ") || text.starts_with("---") {
        return format!("\\{text}");
    }
    // "2026. A year" would start an ordered list: escape the full stop.
    let digits = text.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && text[digits..].starts_with(". ") {
        return format!("{}\\{}", &text[..digits], &text[digits..]);
    }
    text
}

fn render(blocks: &[Block]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut prev_was_item = false;
    for b in blocks {
        let (text, is_item) = match b {
            Block::Heading(level, spans) => {
                (format!("{} {}", "#".repeat(*level), inline(spans, true)), false)
            }
            Block::Paragraph(spans) => (guard_block_start(inline(spans, false)), false),
            Block::Item { marker, spans, .. } => {
                (format!("{} {}", marker.as_deref().unwrap_or("-"), inline(spans, false)), true)
            }
            Block::PageBreak => ("---".to_string(), false),
        };
        if text.trim().is_empty() || text.trim() == "-" {
            continue;
        }
        // List items sit on consecutive lines; everything else is separated
        // by a blank line.
        if !out.is_empty() && !(is_item && prev_was_item) {
            out.push(String::new());
        }
        out.push(text);
        prev_was_item = is_item;
    }
    let mut md = out.join("\n");
    md.push('\n');
    md
}

// ------------------------------------------------------------------ operation

pub fn pdf_to_markdown(
    ctx: &JobContext,
    inputs: &[PathBuf],
    output_name: &str,
    opts: MarkdownOptions,
) -> AppResult<Staged> {
    let input = require_one(inputs)?;
    let name = display_name(input);
    ctx.progress_file(0.05, "Reading document", &name);
    let doc = document::load(input)?;
    let total = doc.get_pages().len() as u32;
    drop(doc);
    let selected = ranges::parse_pages(&opts.pages, total)?;
    let (first, last) = (*selected.iter().min().unwrap_or(&1), *selected.iter().max().unwrap_or(&total));

    let source = ctx.stage_input(input)?;
    let work = ctx.temp.subdir("layout")?;
    let base = work.join("layout");
    ctx.progress(0.2, "Reading the page layout");
    let args: Vec<String> = vec![
        "-xml".into(),
        "-i".into(),
        "-q".into(),
        "-enc".into(),
        "UTF-8".into(),
        "-f".into(),
        first.to_string(),
        "-l".into(),
        last.to_string(),
        source.display().to_string(),
        base.display().to_string(),
    ];
    let out = engines::run(engines::command(Engine::Pdftohtml)?, &args, None, &ctx.cancel, |_| {})?;
    let xml_path = base.with_extension("xml");
    if !out.success() || !xml_path.exists() {
        return Err(AppError::processing(format!("The layout of “{name}” could not be read."))
            .with_detail(out.stderr_tail()));
    }
    ctx.check()?;
    ctx.progress(0.7, "Building Markdown");
    let xml = String::from_utf8_lossy(&fs::read(&xml_path)?).into_owned();
    let wanted: std::collections::HashSet<u32> = selected.iter().copied().collect();
    let pages: Vec<Page> = parse_xml(&xml)?.into_iter().filter(|p| wanted.contains(&p.number)).collect();
    let markdown = to_markdown(&pages, &opts);

    let mut staged = Staged::default();
    if markdown.trim().is_empty() {
        staged.warn("No text was found. The document may be scanned; run OCR PDF on it first.");
    }
    let path = ctx.temp.join("document.md");
    fs::write(&path, markdown)?;
    let base_name = output_name.strip_suffix(".pdf").unwrap_or(output_name);
    staged.files.push((path, file_name_with_extension(base_name, "md")));
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(top: f64, left: f64, size: f64, text: &str) -> Run {
        Run {
            top,
            left,
            width: text.len() as f64 * size * 0.5,
            height: size * 1.2,
            font: Font { size, ..Default::default() },
            spans: vec![Span { text: text.into(), ..Default::default() }],
        }
    }

    fn page(number: u32, runs: Vec<Run>) -> Page {
        Page { number, height: 1000.0, runs }
    }

    #[test]
    fn parses_pdftohtml_xml() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE pdf2xml SYSTEM "pdf2xml.dtd">
<pdf2xml producer="poppler" version="26.01.0">
<page number="1" position="absolute" top="0" left="0" height="1188" width="918">
	<fontspec id="0" size="27" family="Helvetica-Bold" color="#000000"/>
	<fontspec id="1" size="14" family="Helvetica" color="#000000"/>
<text top="72" left="72" width="300" height="31" font="0"><b>Quarterly &amp; Annual</b></text>
<text top="120" left="72" width="400" height="17" font="1">See <a href="https://example.com/x">the site</a> and <i>this</i>&#160;now</text>
</page>
</pdf2xml>"##;
        let pages = parse_xml(xml).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].height, 1188.0);
        let runs = &pages[0].runs;
        assert_eq!(runs.len(), 2);
        assert!(runs[0].font.bold && runs[0].font.size == 27.0);
        assert_eq!(runs[0].spans[0].text, "Quarterly & Annual");
        let spans = &runs[1].spans;
        assert_eq!(spans[1].text, "the site");
        assert_eq!(spans[1].link.as_deref(), Some("https://example.com/x"));
        assert!(spans[3].italic && spans[3].text == "this");
        assert_eq!(spans[4].text, " now");
    }

    #[test]
    fn builds_headings_paragraphs_and_lists() {
        let pages = vec![page(
            1,
            vec![
                run(50.0, 72.0, 28.0, "Annual Report"),
                run(110.0, 72.0, 18.0, "Overview"),
                run(150.0, 72.0, 12.0, "PaperFlow keeps every docu-"),
                run(165.0, 72.0, 12.0, "ment on your computer."),
                run(200.0, 72.0, 12.0, "Second paragraph."),
                run(240.0, 72.0, 12.0, "• First item"),
                run(256.0, 72.0, 12.0, "• Second item that"),
                run(272.0, 84.0, 12.0, "wraps onto a new line"),
                run(300.0, 72.0, 12.0, "1. Numbered"),
            ],
        )];
        let md = to_markdown(&pages, &MarkdownOptions::default());
        assert_eq!(
            md,
            "# Annual Report\n\n## Overview\n\nPaperFlow keeps every document on your computer.\n\n\
             Second paragraph.\n\n- First item\n- Second item that wraps onto a new line\n1. Numbered\n"
        );
    }

    #[test]
    fn headings_can_be_turned_off() {
        let pages =
            vec![page(1, vec![run(50.0, 72.0, 28.0, "Title"), run(100.0, 72.0, 12.0, "Body text here.")])];
        let md = to_markdown(&pages, &MarkdownOptions { detect_headings: false, ..Default::default() });
        assert!(md.starts_with("Title"));
    }

    #[test]
    fn joins_runs_on_one_line_and_keeps_emphasis_and_links() {
        let mut bold = run(100.0, 72.0, 12.0, "Bold");
        bold.spans[0].bold = true;
        let mut linked = run(100.0, 110.0, 12.0, "docs");
        linked.spans[0].link = Some("https://example.com".into());
        let pages = vec![page(1, vec![bold, linked, run(101.0, 150.0, 12.0, "and 2*3_x")])];
        let md = to_markdown(&pages, &MarkdownOptions::default());
        assert_eq!(md, "**Bold** [docs](https://example.com) and 2\\*3\\_x\n");
    }

    #[test]
    fn drops_running_headers_and_page_numbers() {
        let pages: Vec<Page> = (1..=4)
            .map(|n| {
                page(
                    n,
                    vec![
                        run(20.0, 72.0, 10.0, "ACME Confidential"),
                        run(300.0, 72.0, 12.0, &format!("Content of page {n}.")),
                        run(960.0, 300.0, 10.0, &format!("Page {n} of 4")),
                    ],
                )
            })
            .collect();
        let md = to_markdown(&pages, &MarkdownOptions { page_breaks: true, ..Default::default() });
        assert!(!md.contains("ACME"));
        assert!(!md.contains("of 4"));
        assert_eq!(md.matches("Content of page").count(), 4);
        assert_eq!(md.matches("\n---\n").count(), 3);
    }

    #[test]
    fn guards_text_that_looks_like_markdown() {
        let pages = vec![page(
            1,
            vec![run(100.0, 72.0, 12.0, "#hashtag at the start"), run(130.0, 72.0, 12.0, "2026. A year")],
        )];
        let md = to_markdown(&pages, &MarkdownOptions::default());
        assert!(md.contains("\\#hashtag"));
        assert!(md.contains("2026\\. A year"));
    }

    #[test]
    fn recognises_list_markers() {
        assert_eq!(list_marker("• Item"), Some((None, "Item".into())));
        assert_eq!(list_marker("- Item"), Some((None, "Item".into())));
        assert_eq!(list_marker("-5 degrees"), None);
        assert_eq!(list_marker("12) Twelve"), Some((Some("12.".into()), "Twelve".into())));
        assert_eq!(list_marker("2026 was good"), None);
    }
}
