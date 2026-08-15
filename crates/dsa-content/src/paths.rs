//! Finding the content directory on every platform.
//!
//! Content is data that ships *beside* the binary, not inside it, so the
//! lookup has to work for four different layouts: a cargo dev build, a zip
//! unpacked anywhere on Windows, a Linux package split across `/usr/bin` and
//! `/usr/share`, and a macOS `.app` bundle where resources live in
//! `Contents/Resources`.

use std::path::{Path, PathBuf};

/// Environment variable that overrides discovery entirely.
pub const ENV_CONTENT: &str = "DSA_CONTENT";

/// Candidate roots, most specific first. The first one containing
/// `catalog.toml` wins.
pub fn candidate_roots(extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var(ENV_CONTENT) {
        if !p.trim().is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    out.extend_from_slice(extra);

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Windows zip / Linux tarball: content sits next to the binary.
            out.push(dir.join("content"));
            // macOS bundle: MacOS/dsa-visualized -> Resources/content.
            out.push(dir.join("../Resources/content"));
            // Linux prefix install: bin/dsa-visualized -> share/dsa-visualized.
            out.push(dir.join("../share/dsa-visualized/content"));
            // cargo run: target/debug/dsa-visualized -> <repo>/content.
            out.push(dir.join("../../content"));
            out.push(dir.join("../../../content"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("content"));
        out.push(cwd.join("../content"));
    }

    out
}

/// True when `dir` looks like a content root.
pub fn is_content_root(dir: &Path) -> bool {
    dir.join("catalog.toml").is_file()
}

/// Locate the content root, or return the list of places that were tried so
/// the app can show an actionable message instead of an empty window.
pub fn find_content_root(extra: &[PathBuf]) -> Result<PathBuf, Vec<PathBuf>> {
    let candidates = candidate_roots(extra);
    for c in &candidates {
        if is_content_root(c) {
            // Canonicalize so the file watcher and the loader agree on paths.
            return Ok(c.canonicalize().unwrap_or_else(|_| c.clone()));
        }
    }
    Err(candidates)
}

/// `content/problems/<slug>`.
pub fn pack_dir(root: &Path, slug: &str) -> PathBuf {
    root.join("problems").join(slug)
}

/// Turn a problem title into the slug used for its directory and LeetCode URL.
pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if matches!(c, '\'' | '(' | ')' | ',' | '.') {
            // Dropped outright: "Pascal's Triangle" -> "pascals-triangle".
            continue;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_matches_the_leetcode_convention() {
        assert_eq!(slugify("Two Sum"), "two-sum");
        assert_eq!(slugify("Pascal's Triangle"), "pascals-triangle");
        assert_eq!(
            slugify("Two Sum II — Sorted Array"),
            "two-sum-ii-sorted-array"
        );
        assert_eq!(slugify("3Sum"), "3sum");
        assert_eq!(
            slugify("Best Time to Buy & Sell Stock"),
            "best-time-to-buy-sell-stock"
        );
    }

    #[test]
    fn candidates_include_the_exe_relative_layouts() {
        let c = candidate_roots(&[]);
        assert!(c.iter().any(|p| p.ends_with("content")));
        assert!(c.len() >= 3);
    }

    #[test]
    fn explicit_roots_come_first() {
        let extra = vec![PathBuf::from("Z:/explicit")];
        let c = candidate_roots(&extra);
        let pos = c.iter().position(|p| p == &extra[0]).unwrap();
        assert!(pos <= 1, "explicit root should be tried before discovery");
    }
}
