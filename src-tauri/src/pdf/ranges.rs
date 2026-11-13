//! Page range parsing: "1-3, 5, 8-" and friends.

use crate::error::{AppError, AppResult};

/// Parses one comma-separated group list into its groups, each a list of
/// 1-based page numbers in the order written. Supports `n`, `a-b`, `a-`
/// (to the end), `-b` (from the start) and descending ranges such as `5-3`.
/// An empty or "all" specification selects every page as a single group.
pub fn parse_groups(spec: &str, total: u32) -> AppResult<Vec<Vec<u32>>> {
    if total == 0 {
        return Err(AppError::invalid("The document has no pages."));
    }
    let spec = spec.trim();
    if spec.is_empty() || spec.eq_ignore_ascii_case("all") {
        return Ok(vec![(1..=total).collect()]);
    }
    let mut groups = Vec::new();
    for part in spec.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        groups.push(parse_part(part, total)?);
    }
    if groups.is_empty() {
        return Err(AppError::invalid("Enter at least one page or range, for example 1-3, 5."));
    }
    Ok(groups)
}

/// Parses a specification into a flat list of pages (the union of groups,
/// in the order written, duplicates removed).
pub fn parse_pages(spec: &str, total: u32) -> AppResult<Vec<u32>> {
    let mut seen = std::collections::HashSet::new();
    Ok(parse_groups(spec, total)?.into_iter().flatten().filter(|p| seen.insert(*p)).collect())
}

fn parse_part(part: &str, total: u32) -> AppResult<Vec<u32>> {
    let bad = || {
        AppError::invalid(format!("“{part}” is not a valid page range."))
            .with_hint(format!("Use page numbers between 1 and {total}, for example 1-3, 5, 8-."))
    };
    let num = |s: &str| -> AppResult<u32> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("end") || s.eq_ignore_ascii_case("last") {
            return Ok(total);
        }
        s.parse::<u32>().map_err(|_| bad())
    };
    let (start, end) = match part.split_once('-') {
        Some((a, b)) => {
            let a = if a.trim().is_empty() { 1 } else { num(a)? };
            let b = if b.trim().is_empty() { total } else { num(b)? };
            (a, b)
        }
        None => {
            let n = num(part)?;
            (n, n)
        }
    };
    if start == 0 || end == 0 || start > total || end > total {
        return Err(AppError::invalid(format!("“{part}” is outside the document, which has {total} pages."))
            .with_hint(format!("Use page numbers between 1 and {total}.")));
    }
    Ok(if start <= end { (start..=end).collect() } else { (end..=start).rev().collect() })
}

/// Splits `1..=total` into consecutive chunks of `size` pages.
pub fn chunks(total: u32, size: u32) -> AppResult<Vec<Vec<u32>>> {
    if size == 0 {
        return Err(AppError::invalid("The number of pages per file must be at least 1."));
    }
    Ok((1..=total).collect::<Vec<_>>().chunks(size as usize).map(|c| c.to_vec()).collect())
}

/// Compact human description of a page list: [1,2,3,5] -> "1-3, 5".
pub fn describe(pages: &[u32]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < pages.len() {
        let start = pages[i];
        let mut end = start;
        while i + 1 < pages.len() && pages[i + 1] == end + 1 {
            end += 1;
            i += 1;
        }
        parts.push(if start == end { start.to_string() } else { format!("{start}-{end}") });
        i += 1;
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges() {
        assert_eq!(parse_pages("1-3, 5", 10).unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse_pages("8-", 10).unwrap(), vec![8, 9, 10]);
        assert_eq!(parse_pages("-2", 10).unwrap(), vec![1, 2]);
        assert_eq!(parse_pages("3-1", 10).unwrap(), vec![3, 2, 1]);
        assert_eq!(parse_pages("", 3).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_pages("all", 2).unwrap(), vec![1, 2]);
        assert_eq!(parse_pages("2, 2, 1", 3).unwrap(), vec![2, 1]);
        assert_eq!(parse_pages("1-end", 4).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(parse_pages(" 1 ; 3 ", 4).unwrap(), vec![1, 3]);
    }

    #[test]
    fn rejects_bad_ranges() {
        assert!(parse_pages("0", 5).is_err());
        assert!(parse_pages("6", 5).is_err());
        assert!(parse_pages("a-b", 5).is_err());
        assert!(parse_pages("1-99", 5).is_err());
        assert!(parse_pages(",,", 5).is_err());
        assert!(parse_pages("1", 0).is_err());
    }

    #[test]
    fn groups_are_kept_apart() {
        assert_eq!(parse_groups("1-2, 3-4", 4).unwrap(), vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn chunks_pages() {
        assert_eq!(chunks(5, 2).unwrap(), vec![vec![1, 2], vec![3, 4], vec![5]]);
        assert_eq!(chunks(2, 5).unwrap(), vec![vec![1, 2]]);
        assert!(chunks(5, 0).is_err());
    }

    #[test]
    fn describes_pages() {
        assert_eq!(describe(&[1, 2, 3, 5, 7, 8]), "1-3, 5, 7-8");
        assert_eq!(describe(&[4]), "4");
        assert_eq!(describe(&[]), "");
    }
}
