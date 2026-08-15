//! The 📘 helper: data-structure deep-dives and a cross-language syntax cheat
//! sheet, loaded from `content/guide/`.
//!
//! This is teaching material, not code, so it lives in the content tree with
//! everything else — a paragraph can be rewritten and reread without touching
//! the binary.

use dsa_core::problem::LangId;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TopicKind {
    Structure,
    Technique,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OpRow {
    pub op: String,
    pub big: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub kind: TopicKind,
    #[serde(default)]
    pub emoji: String,
    /// Paragraphs explaining how it works.
    #[serde(default)]
    pub what: Vec<String>,
    #[serde(default)]
    pub complexity: Vec<OpRow>,
    /// Per-language syntax sample.
    #[serde(default)]
    pub syntax: BTreeMap<LangId, String>,
    /// Indexing, ranges and pitfalls.
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CheatRow {
    pub topic: String,
    #[serde(flatten)]
    pub code: BTreeMap<LangId, String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CheatSection {
    pub name: String,
    #[serde(default, rename = "row")]
    pub rows: Vec<CheatRow>,
}

#[derive(Debug, Deserialize, Default)]
struct TopicsFile {
    #[serde(default)]
    topic: Vec<Topic>,
}

#[derive(Debug, Deserialize, Default)]
struct CheatFile {
    #[serde(default)]
    section: Vec<CheatSection>,
}

#[derive(Debug, Deserialize, Default)]
struct CategoriesFile {
    #[serde(default)]
    categories: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default)]
pub struct Guide {
    pub topics: Vec<Topic>,
    pub cheatsheet: Vec<CheatSection>,
    /// Problem category -> the topic ids worth reading first.
    pub by_category: BTreeMap<String, Vec<String>>,
    pub errors: Vec<String>,
}

impl Guide {
    /// Load `<root>/guide/`. A missing or broken guide is not fatal: the
    /// helper button simply has less to show.
    pub fn load(root: &Path) -> Self {
        let dir = root.join("guide");
        let mut out = Guide::default();

        // A guide that is not installed is a valid configuration — the helper
        // button just has nothing to show. Only a *malformed* file is an error.
        match read_toml::<TopicsFile>(&dir.join("topics.toml")) {
            Ok(Some(f)) => out.topics = f.topic,
            Ok(None) => return out,
            Err(e) => out.errors.push(e),
        }
        match read_toml::<CheatFile>(&dir.join("cheatsheet.toml")) {
            Ok(Some(f)) => out.cheatsheet = f.section,
            Ok(None) => {}
            Err(e) => out.errors.push(e),
        }
        match read_toml::<CategoriesFile>(&dir.join("categories.toml")) {
            Ok(Some(f)) => out.by_category = f.categories,
            Ok(None) => {}
            Err(e) => out.errors.push(e),
        }

        // A category pointing at a topic that does not exist would silently
        // show nothing, which is exactly the sort of rot the linter exists for.
        let known: Vec<&str> = out.topics.iter().map(|t| t.id.as_str()).collect();
        let mut missing: Vec<String> = Vec::new();
        for (cat, ids) in &out.by_category {
            for id in ids {
                if !known.contains(&id.as_str()) {
                    missing.push(format!(
                        "guide: \"{cat}\" references unknown topic \"{id}\""
                    ));
                }
            }
        }
        out.errors.extend(missing);
        out
    }

    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
    }

    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|t| t.id == id)
    }

    /// Topics relevant to a problem category, in the order the guide lists them.
    pub fn for_category(&self, category: &str) -> Vec<&Topic> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.topic(id)).collect())
            .unwrap_or_default()
    }

    /// Everything not relevant to `category`, for the "everything else" section.
    pub fn rest_for(&self, category: &str) -> Vec<&Topic> {
        let relevant: Vec<&str> = self
            .by_category
            .get(category)
            .map(|ids| ids.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        self.topics
            .iter()
            .filter(|t| !relevant.contains(&t.id.as_str()))
            .collect()
    }
}

/// `Ok(None)` means "not installed"; `Err` means "installed but broken".
fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsa-guide-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("guide")).unwrap();
        std::fs::write(
            dir.join("guide/topics.toml"),
            r#"
[[topic]]
id = 'array'
title = 'Array'
kind = 'structure'
emoji = 'A'
what = ['contiguous memory']
notes = ['zero based']

[topic.syntax]
go = 'nums := []int{}'

[[topic.complexity]]
op = 'index'
big = 'O(1)'

[[topic]]
id = 'twopointers'
title = 'Two Pointers'
kind = 'technique'
what = []
notes = []
[topic.syntax]
go = 'l, r := 0, n-1'
"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("guide/cheatsheet.toml"),
            "[[section]]\nname = 'Basics'\n\n[[section.row]]\ntopic = 'print'\ngo = 'fmt.Println(x)'\ncpp = 'cout << x'\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("guide/categories.toml"),
            "[categories]\n'Arrays & Hashing' = ['array']\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn topics_load_with_their_tables_and_syntax() {
        let g = Guide::load(&fixture("load"));
        assert!(g.errors.is_empty(), "{:?}", g.errors);
        assert_eq!(g.topics.len(), 2);
        let a = g.topic("array").unwrap();
        assert_eq!(a.kind, TopicKind::Structure);
        assert_eq!(a.complexity[0].big, "O(1)");
        assert_eq!(a.syntax.get("go").unwrap(), "nums := []int{}");
        assert_eq!(a.complexity[0].note, "", "an absent note defaults to empty");
    }

    #[test]
    fn a_category_splits_topics_into_relevant_and_the_rest() {
        let g = Guide::load(&fixture("split"));
        let relevant = g.for_category("Arrays & Hashing");
        assert_eq!(relevant.len(), 1);
        assert_eq!(relevant[0].id, "array");
        let rest = g.rest_for("Arrays & Hashing");
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].id, "twopointers");
    }

    #[test]
    fn an_unmapped_category_shows_everything_as_rest() {
        let g = Guide::load(&fixture("unmapped"));
        assert!(g.for_category("Bit Manipulation").is_empty());
        assert_eq!(g.rest_for("Bit Manipulation").len(), 2);
    }

    #[test]
    fn cheat_rows_keep_one_column_per_language() {
        let g = Guide::load(&fixture("cheat"));
        let row = &g.cheatsheet[0].rows[0];
        assert_eq!(row.topic, "print");
        assert_eq!(row.code.get("go").unwrap(), "fmt.Println(x)");
        assert!(
            !row.code.contains_key("java"),
            "a missing language is simply absent"
        );
    }

    #[test]
    fn a_dangling_topic_reference_is_reported() {
        let dir = fixture("dangling");
        std::fs::write(
            dir.join("guide/categories.toml"),
            "[categories]\n'Trees' = ['array', 'nope']\n",
        )
        .unwrap();
        let g = Guide::load(&dir);
        assert!(
            g.errors.iter().any(|e| e.contains("nope")),
            "{:?}",
            g.errors
        );
    }

    #[test]
    fn a_missing_guide_directory_is_not_an_error() {
        // Shipping without the helper is a valid configuration.
        let g = Guide::load(&std::env::temp_dir().join("definitely-no-guide-here"));
        assert!(g.is_empty());
        assert!(g.errors.is_empty(), "{:?}", g.errors);
    }

    #[test]
    fn a_malformed_guide_file_is_an_error() {
        let dir = fixture("broken");
        std::fs::write(
            dir.join("guide/topics.toml"),
            "[[topic]]
id = ",
        )
        .unwrap();
        let g = Guide::load(&dir);
        assert!(
            g.errors.iter().any(|e| e.contains("topics.toml")),
            "{:?}",
            g.errors
        );
    }
}
