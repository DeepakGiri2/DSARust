//! The content library: languages, catalog and problem packs loaded from disk.
//!
//! Loading is deliberately fault-tolerant. A malformed manifest or a script
//! with a syntax error becomes an entry in [`Library::errors`] — the app still
//! starts, still shows every other problem, and shows the failure where the
//! author can act on it. Content authoring is an edit-and-see loop; a hard
//! abort on the first typo would ruin it.

use crate::guide::Guide;
use crate::paths::{self, slugify};
use crate::script::{ScriptError, ScriptHost};
use dsa_core::code::{parse_code, ParsedCode};
use dsa_core::model::Trace;
use dsa_core::problem::{
    CatalogItem, Difficulty, InputMap, LangId, LanguageDef, ProblemMeta, Tier,
};
use rhai::AST;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Manifest shapes
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LanguagesFile {
    #[serde(default)]
    language: Vec<LanguageDef>,
}

#[derive(Debug, Deserialize)]
struct CatalogFile {
    #[serde(default)]
    category: Vec<CatalogCategory>,
}

#[derive(Debug, Deserialize)]
struct CatalogCategory {
    name: String,
    #[serde(default)]
    problems: Vec<CatalogRow>,
}

#[derive(Debug, Deserialize)]
struct CatalogRow {
    title: String,
    difficulty: Difficulty,
    #[serde(default = "row_default_tier")]
    tier: Tier,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    leetcode: Option<String>,
}

fn row_default_tier() -> Tier {
    Tier::T250
}

// ─────────────────────────────────────────────────────────────────────────────
// A loaded problem
// ─────────────────────────────────────────────────────────────────────────────

pub struct ProblemPack {
    pub meta: ProblemMeta,
    pub dir: PathBuf,
    /// Parsed (marker-stripped) source per language id.
    pub sources: BTreeMap<LangId, ParsedCode>,
    /// Raw source with markers, needed by the linter and the porter.
    pub raw_sources: BTreeMap<LangId, String>,
    /// Complete runnable program per language (solution + a `main` that reads
    /// stdin) used to seed the practice editor. Optional: without it the
    /// editor falls back to the bare solution.
    pub practice: BTreeMap<LangId, String>,
    pub script_source: String,
    pub ast: Option<AST>,
    /// Non-fatal problems found while loading this pack.
    pub warnings: Vec<String>,
}

impl ProblemPack {
    pub fn languages(&self) -> impl Iterator<Item = &LangId> {
        self.sources.keys()
    }
    pub fn source(&self, lang: &str) -> Option<&ParsedCode> {
        self.sources.get(lang)
    }
    pub fn has_script(&self) -> bool {
        self.ast.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct LoadError {
    pub slug: String,
    pub file: PathBuf,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.slug, self.message)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The library
// ─────────────────────────────────────────────────────────────────────────────

pub struct Library {
    pub root: PathBuf,
    pub languages: Vec<LanguageDef>,
    pub categories: Vec<String>,
    pub catalog: Vec<CatalogItem>,
    /// The 📘 helper's teaching material.
    pub guide: Guide,
    packs: BTreeMap<String, ProblemPack>,
    host: ScriptHost,
    prelude: String,
    pub errors: Vec<LoadError>,
}

impl Library {
    /// Load everything under `root`. Never fails on bad content — check
    /// [`Library::errors`] afterwards.
    pub fn load(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let mut lib = Library {
            root,
            languages: Vec::new(),
            categories: Vec::new(),
            catalog: Vec::new(),
            guide: Guide::default(),
            packs: BTreeMap::new(),
            host: ScriptHost::new(),
            prelude: String::new(),
            errors: Vec::new(),
        };
        lib.reload_all();
        lib
    }

    pub fn reload_all(&mut self) {
        self.errors.clear();
        self.packs.clear();
        self.load_languages();
        self.load_prelude();
        self.load_catalog();
        self.load_packs();
        self.mark_visualized();
        self.guide = Guide::load(&self.root);
        for e in self.guide.errors.clone() {
            let file = self.root.join("guide");
            self.errors.push(LoadError {
                slug: "guide".into(),
                file,
                message: e,
            });
        }
    }

    // ── manifests ───────────────────────────────────────────────────────────

    fn load_languages(&mut self) {
        let path = self.root.join("languages.toml");
        self.languages = match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<LanguagesFile>(&text) {
                Ok(f) => f.language,
                Err(e) => {
                    self.push_err("languages", &path, e.to_string());
                    Vec::new()
                }
            },
            Err(e) => {
                self.push_err("languages", &path, e.to_string());
                Vec::new()
            }
        };
        self.languages.sort_by_key(|l| (l.order, l.id.clone()));
    }

    fn load_prelude(&mut self) {
        // Every `content/lib/*.rhai` is concatenated ahead of each problem
        // script, so shared helpers need no import ceremony.
        let dir = self.root.join("lib");
        let mut files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "rhai"))
                .collect(),
            Err(_) => Vec::new(),
        };
        files.sort();
        self.prelude = files
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .collect::<Vec<_>>()
            .join("\n");
    }

    fn load_catalog(&mut self) {
        let path = self.root.join("catalog.toml");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.push_err("catalog", &path, e.to_string());
                return;
            }
        };
        let parsed: CatalogFile = match toml::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                self.push_err("catalog", &path, e.to_string());
                return;
            }
        };

        let mut seen: BTreeSet<String> = BTreeSet::new();
        for cat in parsed.category {
            self.categories.push(cat.name.clone());
            for row in cat.problems {
                let slug = row.slug.unwrap_or_else(|| slugify(&row.title));
                if !seen.insert(slug.clone()) {
                    self.push_err(
                        &slug,
                        &path,
                        format!("duplicate catalog entry for \"{}\"", row.title),
                    );
                    continue;
                }
                self.catalog.push(CatalogItem {
                    slug,
                    title: row.title,
                    category: cat.name.clone(),
                    difficulty: row.difficulty,
                    tier: row.tier,
                    leetcode: row.leetcode,
                    viz: false,
                });
            }
        }
    }

    // ── packs ───────────────────────────────────────────────────────────────

    fn load_packs(&mut self) {
        let dir = self.root.join("problems");
        let entries = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => return, // A catalog-only library is legitimate.
        };
        let mut slugs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        slugs.sort();
        for slug in slugs {
            self.load_pack(&slug);
        }
    }

    /// Load (or reload) a single pack. Used by the hot-reload watcher.
    pub fn load_pack(&mut self, slug: &str) {
        let dir = paths::pack_dir(&self.root, slug);
        let meta_path = dir.join("problem.toml");
        let text = match std::fs::read_to_string(&meta_path) {
            Ok(t) => t,
            Err(e) => {
                self.push_err(slug, &meta_path, e.to_string());
                return;
            }
        };
        let mut meta: ProblemMeta = match toml::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                self.push_err(slug, &meta_path, format!("problem.toml: {e}"));
                return;
            }
        };
        if meta.slug.is_empty() {
            meta.slug = slug.to_string();
        }
        if meta.slug != slug {
            self.push_err(
                slug,
                &meta_path,
                format!("slug \"{}\" does not match its directory", meta.slug),
            );
        }

        let mut warnings = Vec::new();
        let (sources, raw_sources) = self.load_sources(&dir, &mut warnings);
        let practice = self.load_practice(&dir);

        // The script is optional: a pack can ship code + metadata only, which
        // is how a problem gets its practice harness before its animation.
        let script_path = dir.join("trace.rhai");
        let script_source = std::fs::read_to_string(&script_path).unwrap_or_default();
        let ast = if script_source.trim().is_empty() {
            None
        } else {
            match self.host.compile(&self.prelude, &script_source) {
                Ok(ast) => Some(ast),
                Err(e) => {
                    self.errors.push(LoadError {
                        slug: slug.to_string(),
                        file: script_path.clone(),
                        message: strip_prelude_lines(&e, &self.prelude),
                    });
                    None
                }
            }
        };

        self.packs.insert(
            slug.to_string(),
            ProblemPack {
                meta,
                dir,
                sources,
                raw_sources,
                practice,
                script_source,
                ast,
                warnings,
            },
        );
    }

    fn load_practice(&self, dir: &Path) -> BTreeMap<LangId, String> {
        let practice_dir = dir.join("practice");
        self.languages
            .iter()
            .filter_map(|lang| {
                std::fs::read_to_string(practice_dir.join(format!("{}.txt", lang.id)))
                    .ok()
                    .map(|src| (lang.id.clone(), src))
            })
            .collect()
    }

    fn load_sources(
        &self,
        dir: &Path,
        warnings: &mut Vec<String>,
    ) -> (BTreeMap<LangId, ParsedCode>, BTreeMap<LangId, String>) {
        let mut parsed = BTreeMap::new();
        let mut raw = BTreeMap::new();
        let code_dir = dir.join("code");
        for lang in &self.languages {
            let path = code_dir.join(format!("{}.txt", lang.id));
            if let Ok(src) = std::fs::read_to_string(&path) {
                parsed.insert(lang.id.clone(), parse_code(&src, &lang.comment));
                raw.insert(lang.id.clone(), src);
            }
        }
        if parsed.is_empty() {
            warnings.push("no solution sources found in code/".into());
        }
        (parsed, raw)
    }

    fn mark_visualized(&mut self) {
        for item in &mut self.catalog {
            item.viz = self.packs.get(&item.slug).is_some_and(|p| p.has_script());
        }
        // A pack with no catalog row would be invisible in the UI — surface it
        // rather than letting the author wonder where their problem went.
        let known: BTreeSet<&str> = self.catalog.iter().map(|c| c.slug.as_str()).collect();
        let orphans: Vec<String> = self
            .packs
            .keys()
            .filter(|s| !known.contains(s.as_str()))
            .cloned()
            .collect();
        for slug in orphans {
            let file = paths::pack_dir(&self.root, &slug);
            self.errors.push(LoadError {
                slug: slug.clone(),
                file,
                message: "pack has no catalog.toml entry, so it will not be listed".into(),
            });
        }
    }

    fn push_err(&mut self, slug: &str, file: &Path, message: String) {
        self.errors.push(LoadError {
            slug: slug.to_string(),
            file: file.to_path_buf(),
            message,
        });
    }

    // ── queries ─────────────────────────────────────────────────────────────

    pub fn pack(&self, slug: &str) -> Option<&ProblemPack> {
        self.packs.get(slug)
    }
    pub fn packs(&self) -> impl Iterator<Item = (&String, &ProblemPack)> {
        self.packs.iter()
    }
    pub fn item(&self, slug: &str) -> Option<&CatalogItem> {
        self.catalog.iter().find(|c| c.slug == slug)
    }
    pub fn language(&self, id: &str) -> Option<&LanguageDef> {
        self.languages.iter().find(|l| l.id == id)
    }

    /// Count of catalog rows within a tier, and how many of those are animated.
    pub fn tier_progress(&self, tier: Tier) -> (usize, usize) {
        let items: Vec<&CatalogItem> = self
            .catalog
            .iter()
            .filter(|c| tier.contains(c.tier))
            .collect();
        (items.iter().filter(|c| c.viz).count(), items.len())
    }

    /// Record a trace for `slug` with the given input.
    pub fn trace(&self, slug: &str, input: &InputMap) -> Result<Trace, ScriptError> {
        let pack = self
            .packs
            .get(slug)
            .ok_or_else(|| ScriptError::Runtime(format!("no content pack for \"{slug}\"")))?;
        let ast = pack
            .ast
            .as_ref()
            .ok_or_else(|| ScriptError::Runtime(format!("\"{slug}\" has no trace.rhai yet")))?;
        self.host.run_trace(ast, input).map_err(|e| match e {
            ScriptError::Runtime(m) => {
                ScriptError::Runtime(strip_prelude_lines_str(&m, &self.prelude))
            }
            other => other,
        })
    }

    /// Script-side validation, on top of the manifest's declarative rules.
    pub fn validate(&self, slug: &str, input: &InputMap) -> Option<String> {
        let pack = self.packs.get(slug)?;
        let ast = pack.ast.as_ref()?;
        self.host.run_validate(ast, input)
    }

    pub fn prelude(&self) -> &str {
        &self.prelude
    }
    pub fn host(&self) -> &ScriptHost {
        &self.host
    }
}

/// Rhai reports positions in the concatenated (prelude + script) source. Shift
/// them back so an author's line 12 is reported as line 12.
fn strip_prelude_lines(e: &ScriptError, prelude: &str) -> String {
    strip_prelude_lines_str(&e.to_string(), prelude)
}

fn strip_prelude_lines_str(msg: &str, prelude: &str) -> String {
    if prelude.is_empty() {
        return msg.to_string();
    }
    let offset = prelude.lines().count() + 1;
    // Rhai formats positions as "(line 42, position 7)".
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(pos) = rest.find("(line ") {
        out.push_str(&rest[..pos + 6]);
        rest = &rest[pos + 6..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        match rest[..end].parse::<usize>() {
            Ok(n) => out.push_str(&n.saturating_sub(offset).max(1).to_string()),
            Err(_) => out.push_str(&rest[..end]),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::InputValue;

    /// Builds a tiny but complete content root in a temp dir. `name` keeps
    /// tests isolated — they run in parallel and each one mutates its tree.
    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsa-lib-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("problems/two-sum/code")).unwrap();
        std::fs::create_dir_all(dir.join("lib")).unwrap();

        std::fs::write(
            dir.join("languages.toml"),
            r##"
[[language]]
id = "go"
label = "Go"
ext = "go"
order = 0
[[language]]
id = "python"
label = "Python"
ext = "py"
comment = "#"
order = 1
"##,
        )
        .unwrap();

        std::fs::write(
            dir.join("catalog.toml"),
            r#"
[[category]]
name = "Arrays & Hashing"
problems = [
  { title = "Two Sum", difficulty = "Easy", tier = "50" },
  { title = "Group Anagrams", difficulty = "Medium", tier = "50" },
]
"#,
        )
        .unwrap();

        std::fs::write(dir.join("lib/common.rhai"), "fn twice(x) { x * 2 }\n").unwrap();

        std::fs::write(
            dir.join("problems/two-sum/problem.toml"),
            r#"
slug = "two-sum"
title = "Two Sum"
category = "Arrays & Hashing"
difficulty = "Easy"
tier = "50"
description = "Find two indices."
complexity = "O(n) time"

[[inputs]]
name = "nums"
type = "int-array"

[default_input]
nums = [2, 7, 11]

[[tests]]
expected = "0 1"
[tests.input]
nums = [2, 7]
"#,
        )
        .unwrap();

        std::fs::write(
            dir.join("problems/two-sum/code/go.txt"),
            "func twoSum() {\n    seen := map[int]int{} //@init\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("problems/two-sum/code/python.txt"),
            "def two_sum():\n    seen = {}  #@init\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("problems/two-sum/trace.rhai"),
            r#"fn trace(input) {
                enter("twoSum");
                step("init", `n = ${twice(input.nums.len())}`, [ array("nums", input.nums) ]);
            }"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn loads_languages_catalog_and_packs() {
        let dir = fixture("load");
        let lib = Library::load(&dir);
        assert!(lib.errors.is_empty(), "{:?}", lib.errors);
        assert_eq!(lib.languages.len(), 2);
        assert_eq!(lib.catalog.len(), 2);
        assert_eq!(lib.categories, vec!["Arrays & Hashing".to_string()]);
        assert!(lib.item("two-sum").unwrap().viz);
        assert!(!lib.item("group-anagrams").unwrap().viz);
        assert_eq!(lib.tier_progress(Tier::T50), (1, 2));
    }

    #[test]
    fn per_language_comment_markers_are_honoured() {
        let lib = Library::load(fixture("comments"));
        let pack = lib.pack("two-sum").unwrap();
        assert_eq!(pack.source("go").unwrap().line_of("init"), Some(2));
        assert_eq!(pack.source("python").unwrap().line_of("init"), Some(2));
        assert!(!pack.source("python").unwrap().clean.contains("@init"));
    }

    #[test]
    fn prelude_helpers_are_available_and_traces_run() {
        let lib = Library::load(fixture("prelude"));
        let mut input = InputMap::new();
        input.insert(
            "nums".into(),
            InputValue::List(vec![InputValue::Int(2), InputValue::Int(7)]),
        );
        let trace = lib.trace("two-sum", &input).unwrap();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace.steps[0].note, "n = 4");
    }

    #[test]
    fn a_broken_pack_does_not_stop_the_library() {
        let dir = fixture("broken");
        std::fs::create_dir_all(dir.join("problems/broken")).unwrap();
        std::fs::write(dir.join("problems/broken/problem.toml"), "slug = ").unwrap();
        let lib = Library::load(&dir);
        assert!(lib.pack("two-sum").is_some(), "good pack still loaded");
        assert!(lib.errors.iter().any(|e| e.slug == "broken"));
    }

    #[test]
    fn a_pack_without_a_catalog_row_is_reported() {
        let dir = fixture("orphan");
        std::fs::create_dir_all(dir.join("problems/ghost/code")).unwrap();
        std::fs::write(
            dir.join("problems/ghost/problem.toml"),
            "slug = \"ghost\"\ntitle = \"Ghost\"\ncategory = \"X\"\ndifficulty = \"Easy\"\n",
        )
        .unwrap();
        let lib = Library::load(&dir);
        assert!(lib
            .errors
            .iter()
            .any(|e| e.slug == "ghost" && e.message.contains("catalog")));
    }

    #[test]
    fn script_errors_are_reported_against_the_authors_line_numbers() {
        let dir = fixture("lines");
        // Line 2 of the problem script is the bad one.
        std::fs::write(
            dir.join("problems/two-sum/trace.rhai"),
            "fn trace(input) {\n    let x = ;\n}\n",
        )
        .unwrap();
        let lib = Library::load(&dir);
        let e = lib
            .errors
            .iter()
            .find(|e| e.slug == "two-sum")
            .expect("reported");
        assert!(
            e.message.contains("line 2"),
            "prelude offset not removed: {}",
            e.message
        );
    }

    #[test]
    fn duplicate_catalog_entries_are_caught() {
        let dir = fixture("dupes");
        std::fs::write(
            dir.join("catalog.toml"),
            r#"
[[category]]
name = "A"
problems = [
  { title = "Two Sum", difficulty = "Easy", tier = "50" },
  { title = "Two Sum", difficulty = "Easy", tier = "50" },
]
"#,
        )
        .unwrap();
        let lib = Library::load(&dir);
        assert_eq!(lib.catalog.len(), 1);
        assert!(lib.errors.iter().any(|e| e.message.contains("duplicate")));
    }

    #[test]
    fn missing_content_root_yields_errors_not_a_panic() {
        let lib = Library::load(std::env::temp_dir().join("definitely-not-content-xyz"));
        assert!(!lib.errors.is_empty());
        assert!(lib.catalog.is_empty());
    }
}
