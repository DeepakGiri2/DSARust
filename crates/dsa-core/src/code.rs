//! Source annotation: mapping one logical trace step to the right line in
//! every language.
//!
//! Problem sources carry trailing markers — `//@loop`, `#@loop` — that name the
//! step a line belongs to. Parsing strips them for display and records
//! `tag -> line`, which is how the same step highlights line 6 of the Go source
//! and line 4 of the Python one.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct ParsedCode {
    /// Source with markers removed, ready to display.
    pub clean: String,
    /// Marker name to 1-based line number.
    pub tag_to_line: BTreeMap<String, usize>,
    /// Line number to marker name, for gutter breakpoints.
    pub line_to_tag: BTreeMap<usize, String>,
}

impl ParsedCode {
    pub fn line_of(&self, tag: &str) -> Option<usize> {
        self.tag_to_line.get(tag).copied()
    }
    pub fn tag_at(&self, line: usize) -> Option<&str> {
        self.line_to_tag.get(&line).map(|s| s.as_str())
    }
    pub fn line_count(&self) -> usize {
        self.clean.lines().count()
    }
}

/// Strip `<comment>@tag` markers and record where each one landed.
///
/// `comment` is the language's line-comment prefix (`//`, `#`, `--`). A marker
/// must be the last thing on the line; anything else is left alone, so a real
/// comment such as `// @ts-ignore` is never mistaken for a marker.
pub fn parse_code(source: &str, comment: &str) -> ParsedCode {
    let mut tag_to_line = BTreeMap::new();
    let mut line_to_tag = BTreeMap::new();
    let mut clean = String::with_capacity(source.len());

    for (i, line) in source.lines().enumerate() {
        match split_marker(line, comment) {
            Some((body, tag)) => {
                let lineno = i + 1;
                // First occurrence wins: a tag reused further down still
                // highlights the line the step was authored against.
                tag_to_line.entry(tag.to_string()).or_insert(lineno);
                line_to_tag.insert(lineno, tag.to_string());
                clean.push_str(body.trim_end());
            }
            None => clean.push_str(line),
        }
        clean.push('\n');
    }
    if !source.ends_with('\n') {
        clean.pop();
    }

    ParsedCode {
        clean,
        tag_to_line,
        line_to_tag,
    }
}

/// Returns `(line_without_marker, tag)` when the line ends in `<comment>@name`.
fn split_marker<'a>(line: &'a str, comment: &str) -> Option<(&'a str, &'a str)> {
    let trimmed = line.trim_end();
    let at = trimmed.rfind(comment)?;
    let rest = &trimmed[at + comment.len()..];
    let tag = rest.strip_prefix('@')?;
    if tag.is_empty()
        || !tag
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((&line[..at], tag))
}

/// Every tag a source declares, in line order.
pub fn tags_in_order(parsed: &ParsedCode) -> Vec<&str> {
    let mut v: Vec<(usize, &str)> = parsed
        .line_to_tag
        .iter()
        .map(|(l, t)| (*l, t.as_str()))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, t)| t).collect()
}

/// Tags a trace uses that a given source never declares. Surfaced by the
/// content linter — a missing tag means the highlight silently sticks on the
/// previous line, which is exactly the sort of rot that goes unnoticed.
pub fn missing_tags<'a>(parsed: &ParsedCode, used: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut miss: Vec<String> = used
        .filter(|t| !parsed.tag_to_line.contains_key(*t))
        .map(|t| t.to_string())
        .collect();
    miss.sort();
    miss.dedup();
    miss
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO: &str = "func twoSum(nums []int) []int {\n    seen := map[int]int{} //@init\n    for i := range nums { //@loop\n        _ = i\n    }\n    return nil //@none\n}";

    #[test]
    fn markers_are_stripped_and_located() {
        let p = parse_code(GO, "//");
        assert_eq!(p.line_of("init"), Some(2));
        assert_eq!(p.line_of("loop"), Some(3));
        assert_eq!(p.line_of("none"), Some(6));
        assert!(!p.clean.contains("@init"));
        assert!(p.clean.contains("seen := map[int]int{}"));
        assert_eq!(p.clean.lines().count(), GO.lines().count());
    }

    #[test]
    fn trailing_whitespace_before_the_marker_goes_too() {
        let p = parse_code("x := 1   //@a", "//");
        assert_eq!(p.clean.trim_end(), "x := 1");
    }

    #[test]
    fn python_style_comments_work() {
        let p = parse_code(
            "seen = {}  #@init\nfor i, x in enumerate(nums):  #@loop",
            "#",
        );
        assert_eq!(p.line_of("init"), Some(1));
        assert_eq!(p.line_of("loop"), Some(2));
        assert_eq!(p.clean.lines().next().unwrap(), "seen = {}");
    }

    #[test]
    fn ordinary_comments_are_not_markers() {
        let p = parse_code("x := 1 // just a note\ny := 2 //@tag", "//");
        assert!(p.tag_to_line.contains_key("tag"));
        assert_eq!(p.tag_to_line.len(), 1);
        assert!(p.clean.contains("// just a note"));
    }

    #[test]
    fn a_url_inside_a_string_is_not_a_marker() {
        let p = parse_code("s := \"https://x@y\"", "//");
        assert!(p.tag_to_line.is_empty());
        assert_eq!(p.clean.trim(), "s := \"https://x@y\"");
    }

    #[test]
    fn line_lookup_round_trips_for_the_gutter() {
        let p = parse_code(GO, "//");
        assert_eq!(p.tag_at(3), Some("loop"));
        assert_eq!(p.tag_at(4), None);
    }

    #[test]
    fn missing_tags_are_reported_sorted() {
        let p = parse_code(GO, "//");
        let miss = missing_tags(&p, ["init", "store", "check", "store"].into_iter());
        assert_eq!(miss, vec!["check", "store"]);
    }
}
