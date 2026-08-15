//! Problem metadata, the language registry and input validation.
//!
//! Two deliberate design choices here, both in service of "adding content must
//! not mean rebuilding the app":
//!
//! * **Languages are data.** There is no `enum Lang { Go, Cpp, Java }`. A
//!   language is a [`LanguageDef`] loaded from `content/languages.toml`, so
//!   shipping Python or Rust solutions is a content edit, not a code change.
//! * **Validation is declarative.** The common constraints (bounds, length,
//!   charset, sortedness) live in the problem manifest. Scripts only need a
//!   `validate` function for genuinely bespoke rules.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type LangId = String;

/// Everything the app needs to display and run one language, defined in
/// `content/languages.toml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageDef {
    pub id: LangId,
    pub label: String,
    /// Extension used for the source file the harness writes, e.g. `go`.
    pub ext: String,
    /// Highlighter hint, e.g. `c++`, `go`, `java`, `python`.
    #[serde(default)]
    pub syntax: String,
    /// Line-comment prefix that introduces a `@tag` marker.
    #[serde(default = "default_comment")]
    pub comment: String,
    /// Display order in the language tab strip.
    #[serde(default)]
    pub order: i32,
    /// How to build and run a program locally. Absent = viewer-only language.
    #[serde(default)]
    pub toolchain: Option<Toolchain>,
    /// Compiler id for the remote Compiler Explorer fallback.
    #[serde(default)]
    pub remote_compiler: Option<String>,
    /// Some toolchains insist the file be named after the entry class (Java).
    #[serde(default)]
    pub source_name: Option<String>,
}

fn default_comment() -> String {
    "//".into()
}

/// Argv templates for building and running a submission. `{src}`, `{exe}` and
/// `{dir}` are substituted; no shell is involved, so quoting and spaces in
/// paths behave identically on every OS.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Toolchain {
    /// Probe binary that must exist for this language to be locally runnable.
    pub probe: String,
    #[serde(default)]
    pub compile: Vec<String>,
    pub run: Vec<String>,
    /// Windows tends to name the same tool differently (`g++` vs `g++.exe` is
    /// handled by `which`, but `python3` vs `python` is not).
    #[serde(default)]
    pub probe_windows: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn short(&self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Med",
            Difficulty::Hard => "Hard",
        }
    }
}

/// Smallest problem list that contains a problem.
///
/// The first three are the NeetCode roadmaps, nested. `Extra` sits outside
/// them: high-frequency interview problems and whole patterns the roadmap
/// never covers (union–find, prefix sums, string matching, design). It is
/// last so that widening the filter to it shows everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    #[serde(rename = "50")]
    T50,
    #[serde(rename = "150")]
    T150,
    #[serde(rename = "250")]
    T250,
    #[serde(rename = "extra")]
    Extra,
}

impl Tier {
    pub fn label(&self) -> &'static str {
        match self {
            Tier::T50 => "50",
            Tier::T150 => "150",
            Tier::T250 => "250",
            Tier::Extra => "extra",
        }
    }
    /// How the tier names itself in the UI — the roadmap tiers carry the
    /// roadmap's name, the extra tier does not pretend to.
    pub fn title(&self) -> &'static str {
        match self {
            Tier::T50 => "NeetCode 50",
            Tier::T150 => "NeetCode 150",
            Tier::T250 => "NeetCode 250",
            Tier::Extra => "+ Interview Extra",
        }
    }
    /// A problem shows in a tier if it belongs to that tier or a smaller one.
    pub fn contains(&self, item: Tier) -> bool {
        item <= *self
    }
    pub const ALL: [Tier; 4] = [Tier::T50, Tier::T150, Tier::T250, Tier::Extra];
}

// ─────────────────────────────────────────────────────────────────────────────
// Inputs
// ─────────────────────────────────────────────────────────────────────────────

/// A dynamic input value. Recursive so grids and nested lists need no special
/// case; `Null` exists for level-order tree encodings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<InputValue>),
    Null,
}

impl InputValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            InputValue::Int(i) => Some(*i),
            InputValue::Float(f) => Some(*f as i64),
            InputValue::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            InputValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_list(&self) -> Option<&[InputValue]> {
        match self {
            InputValue::List(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_int_list(&self) -> Option<Vec<i64>> {
        self.as_list()
            .map(|v| v.iter().filter_map(|x| x.as_i64()).collect())
    }

    /// Text form shown in the input editor.
    pub fn to_editable(&self) -> String {
        match self {
            InputValue::Bool(b) => b.to_string(),
            InputValue::Int(i) => i.to_string(),
            InputValue::Float(f) => f.to_string(),
            InputValue::Str(s) => s.clone(),
            InputValue::Null => "null".into(),
            InputValue::List(v) => v
                .iter()
                .map(|x| x.to_editable())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

pub type InputMap = BTreeMap<String, InputValue>;

/// The names here are the ones authors type in `problem.toml`, so they read
/// the way a person would say them (`string`, not `str`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputType {
    Int,
    Float,
    Bool,
    IntArray,
    /// Whitespace-separated words.
    #[serde(rename = "string-array")]
    StrArray,
    #[serde(rename = "string")]
    Str,
    /// Level-order tree text, `n` or `null` for a missing child.
    Tree,
    /// Rows of digits/letters separated by whitespace.
    Grid,
}

/// One editable input, with the mechanical constraints attached.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputField {
    pub name: String,
    #[serde(default)]
    pub label: String,
    #[serde(rename = "type")]
    pub ty: InputType,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub min_len: Option<usize>,
    #[serde(default)]
    pub max_len: Option<usize>,
    /// Allowed characters for `Str`/`Grid` inputs, e.g. `"01"` or `"a-z"`.
    #[serde(default)]
    pub charset: Option<String>,
    /// Require a non-decreasing `IntArray`.
    #[serde(default)]
    pub sorted: bool,
    /// Require distinct values in an `IntArray`.
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub help: Option<String>,
}

impl InputField {
    pub fn display_label(&self) -> &str {
        if self.label.is_empty() {
            &self.name
        } else {
            &self.label
        }
    }

    /// Parse the editor's text into a value of this field's type.
    pub fn parse(&self, text: &str) -> Result<InputValue, String> {
        let t = text.trim();
        match self.ty {
            InputType::Int => t
                .parse::<i64>()
                .map(InputValue::Int)
                .map_err(|_| format!("{} must be a whole number", self.display_label())),
            InputType::Float => t
                .parse::<f64>()
                .map(InputValue::Float)
                .map_err(|_| format!("{} must be a number", self.display_label())),
            InputType::Bool => match t {
                "true" | "1" | "yes" => Ok(InputValue::Bool(true)),
                "false" | "0" | "no" => Ok(InputValue::Bool(false)),
                _ => Err(format!("{} must be true or false", self.display_label())),
            },
            InputType::IntArray => {
                let mut out = Vec::new();
                for tok in t.split([' ', ',', '\t', '\n']).filter(|s| !s.is_empty()) {
                    match tok.parse::<i64>() {
                        Ok(v) => out.push(InputValue::Int(v)),
                        Err(_) => {
                            return Err(format!(
                                "{}: \"{tok}\" is not a whole number",
                                self.display_label()
                            ))
                        }
                    }
                }
                Ok(InputValue::List(out))
            }
            InputType::StrArray => Ok(InputValue::List(
                t.split_whitespace()
                    .map(|s| InputValue::Str(s.to_string()))
                    .collect(),
            )),
            InputType::Str | InputType::Tree | InputType::Grid => {
                Ok(InputValue::Str(text.trim_end_matches('\n').to_string()))
            }
        }
    }

    /// Mechanical constraint check. Returns the first violation, phrased for a
    /// human rather than a parser.
    pub fn validate(&self, v: &InputValue) -> Option<String> {
        let name = self.display_label();
        match self.ty {
            InputType::Int | InputType::Float => {
                let n = v.as_i64()?;
                if let Some(min) = self.min {
                    if n < min {
                        return Some(format!("{name} must be at least {min}"));
                    }
                }
                if let Some(max) = self.max {
                    if n > max {
                        return Some(format!("{name} must be at most {max}"));
                    }
                }
            }
            InputType::IntArray => {
                let items = v.as_int_list()?;
                if let Some(m) = self.min_len {
                    if items.len() < m {
                        return Some(format!("{name} needs at least {m} value(s)"));
                    }
                }
                if let Some(m) = self.max_len {
                    if items.len() > m {
                        return Some(format!(
                            "{name} is limited to {m} values so the animation stays readable"
                        ));
                    }
                }
                if let Some(min) = self.min {
                    if items.iter().any(|x| *x < min) {
                        return Some(format!("every value in {name} must be at least {min}"));
                    }
                }
                if let Some(max) = self.max {
                    if items.iter().any(|x| *x > max) {
                        return Some(format!("every value in {name} must be at most {max}"));
                    }
                }
                if self.sorted && items.windows(2).any(|w| w[0] > w[1]) {
                    return Some(format!("{name} must be sorted in non-decreasing order"));
                }
                if self.unique {
                    let mut seen = items.clone();
                    seen.sort_unstable();
                    seen.dedup();
                    if seen.len() != items.len() {
                        return Some(format!("{name} must not contain duplicates"));
                    }
                }
            }
            InputType::Str | InputType::Tree | InputType::Grid | InputType::StrArray => {
                let s = match v {
                    InputValue::Str(s) => s.clone(),
                    InputValue::List(_) => v.to_editable(),
                    _ => return None,
                };
                if let Some(m) = self.min_len {
                    if s.trim().len() < m {
                        return Some(format!("{name} needs at least {m} character(s)"));
                    }
                }
                if let Some(m) = self.max_len {
                    if s.trim().len() > m {
                        return Some(format!(
                            "{name} is limited to {m} characters so the animation stays readable"
                        ));
                    }
                }
                if let Some(cs) = &self.charset {
                    let allowed = expand_charset(cs);
                    if let Some(bad) = s
                        .chars()
                        .find(|c| !c.is_whitespace() && !allowed.contains(*c))
                    {
                        return Some(format!("{name}: '{bad}' is not allowed here ({cs})"));
                    }
                }
                if self.ty == InputType::Grid {
                    let rows: Vec<&str> = s.split_whitespace().collect();
                    if let Some(first) = rows.first() {
                        if rows
                            .iter()
                            .any(|r| r.chars().count() != first.chars().count())
                        {
                            return Some(format!("{name}: every row must be the same width"));
                        }
                    }
                }
            }
            InputType::Bool => {}
        }
        None
    }
}

/// Expands `"a-z0-9_"` style shorthand into the concrete character set.
fn expand_charset(spec: &str) -> String {
    let chars: Vec<char> = spec.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let (a, b) = (chars[i], chars[i + 2]);
            if a <= b {
                for c in a..=b {
                    out.push(c);
                }
                i += 3;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Validate every field of an input map. Returns all violations so the editor
/// can flag more than one bad field at a time.
pub fn validate_inputs(fields: &[InputField], values: &InputMap) -> Vec<String> {
    let mut errs = Vec::new();
    for f in fields {
        match values.get(&f.name) {
            Some(v) => {
                if let Some(e) = f.validate(v) {
                    errs.push(e);
                }
            }
            None => errs.push(format!("missing input \"{}\"", f.name)),
        }
    }
    errs
}

// ─────────────────────────────────────────────────────────────────────────────
// Problem + catalog metadata
// ─────────────────────────────────────────────────────────────────────────────

/// A test case: an input map plus the expected stdout of the harness program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestCase {
    #[serde(default)]
    pub name: String,
    pub input: InputMap,
    pub expected: String,
    /// Marks a deliberately tricky case in the results table.
    #[serde(default)]
    pub edge: bool,
}

/// `problem.toml`, deserialized. The trace script and code sources live beside
/// it as separate files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProblemMeta {
    pub slug: String,
    pub title: String,
    pub category: String,
    pub difficulty: Difficulty,
    #[serde(default = "default_tier")]
    pub tier: Tier,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub approach: String,
    #[serde(default)]
    pub complexity: String,
    /// LeetCode slug when it differs from ours (e.g. `3sum`).
    #[serde(default)]
    pub leetcode: Option<String>,
    #[serde(default)]
    pub inputs: Vec<InputField>,
    #[serde(default)]
    pub default_input: InputMap,
    #[serde(default)]
    pub tests: Vec<TestCase>,
    /// Extra reading shown in the explanation drawer.
    #[serde(default)]
    pub hints: Vec<String>,
    /// Related problem slugs, surfaced as "practice next".
    #[serde(default)]
    pub related: Vec<String>,
}

fn default_tier() -> Tier {
    Tier::T250
}

impl ProblemMeta {
    pub fn leetcode_url(&self) -> String {
        let slug = self.leetcode.as_deref().unwrap_or(&self.slug);
        format!("https://leetcode.com/problems/{slug}/")
    }
}

/// A catalog row. Every NeetCode problem has one; `viz` says whether an
/// interactive pack was found for it on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogItem {
    pub slug: String,
    pub title: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub tier: Tier,
    #[serde(default)]
    pub leetcode: Option<String>,
    #[serde(skip)]
    pub viz: bool,
}

impl CatalogItem {
    pub fn leetcode_url(&self) -> String {
        let slug = self.leetcode.as_deref().unwrap_or(&self.slug);
        format!("https://leetcode.com/problems/{slug}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(ty: InputType) -> InputField {
        InputField {
            name: "nums".into(),
            label: String::new(),
            ty,
            min: None,
            max: None,
            min_len: None,
            max_len: None,
            charset: None,
            sorted: false,
            unique: false,
            help: None,
        }
    }

    #[test]
    fn int_array_accepts_commas_and_spaces() {
        let f = field(InputType::IntArray);
        assert_eq!(
            f.parse("1, 2  3").unwrap().as_int_list().unwrap(),
            vec![1, 2, 3]
        );
        assert!(f.parse("1 x").is_err());
    }

    #[test]
    fn sorted_and_unique_constraints_report_plainly() {
        let mut f = field(InputType::IntArray);
        f.sorted = true;
        let v = f.parse("3 1").unwrap();
        assert!(f.validate(&v).unwrap().contains("sorted"));

        let mut f = field(InputType::IntArray);
        f.unique = true;
        let v = f.parse("1 1").unwrap();
        assert!(f.validate(&v).unwrap().contains("duplicates"));
    }

    #[test]
    fn charset_ranges_expand() {
        assert_eq!(expand_charset("a-e"), "abcde");
        assert_eq!(expand_charset("01"), "01");
        assert_eq!(expand_charset("a-c0-1_"), "abc01_");
    }

    #[test]
    fn grid_rows_must_be_rectangular() {
        let mut f = field(InputType::Grid);
        f.charset = Some("01".into());
        let ok = f.parse("110 011").unwrap();
        assert!(f.validate(&ok).is_none());
        let ragged = f.parse("110 01").unwrap();
        assert!(f.validate(&ragged).unwrap().contains("same width"));
        let bad = f.parse("11a 011").unwrap();
        assert!(f.validate(&bad).unwrap().contains("not allowed"));
    }

    #[test]
    fn tier_containment_follows_the_neetcode_nesting() {
        assert!(Tier::T250.contains(Tier::T50));
        assert!(Tier::T150.contains(Tier::T50));
        assert!(!Tier::T50.contains(Tier::T150));
    }

    #[test]
    fn the_extra_tier_sits_outside_the_roadmap_and_widens_to_everything() {
        // Widening to Extra shows the roadmap too …
        for t in Tier::ALL {
            assert!(Tier::Extra.contains(t));
        }
        // … but an extra problem never leaks into a roadmap tier.
        assert!(!Tier::T250.contains(Tier::Extra));
        assert_eq!(
            serde_json::from_str::<Tier>("\"extra\"").unwrap(),
            Tier::Extra,
            "catalog rows spell it tier = 'extra'"
        );
    }

    #[test]
    fn missing_input_is_reported_not_panicked() {
        let fields = vec![field(InputType::IntArray)];
        let errs = validate_inputs(&fields, &InputMap::new());
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("missing"));
    }
}
