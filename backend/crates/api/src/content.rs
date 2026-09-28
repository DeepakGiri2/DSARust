//! Problem content, loaded once per process from `content/` with the desktop's
//! own loader (`dsa_content::Library`), so the cloud serves byte-for-byte what
//! the desktop shows.
//!
//! Content is immutable for the life of a deploy — it is baked into the image
//! — so every public payload (catalog, guide, each problem) is built once at
//! startup and served from memory, and a hash of the whole tree is the
//! `content_version` that keys ETags, CDN entries and trace caches. A new
//! content release is a new image, which is a new version, which busts every
//! cache at once and never half of them.

use crate::dto;
use anyhow::{bail, Result};
use dsa_content::{Library, ProblemPack};
use dsa_core::problem::{CatalogItem, InputMap, LanguageDef, Tier};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

pub struct ContentStore {
    pub lib: Library,
    pub version: String,
    /// Tiers that need the Pro plan (from `PREMIUM_TIERS`).
    premium_tiers: Vec<Tier>,
    catalog: Arc<dto::Catalog>,
    catalog_json: Arc<Vec<u8>>,
    guide_json: Arc<Vec<u8>>,
    /// Full problem payloads, sources included.
    problems: BTreeMap<String, Arc<dto::Problem>>,
    slugs: BTreeSet<String>,
    runnable: BTreeSet<String>,
}

impl ContentStore {
    /// Load and pre-render everything. Unlike the desktop — which tolerates a
    /// broken pack so an author can keep editing — a server refuses to start
    /// on bad content: a deploy that silently drops problems is worse than one
    /// that fails its health check and rolls back.
    pub fn load(root: &Path, premium_tiers: Vec<Tier>, runnable: BTreeSet<String>) -> Result<Self> {
        if !root.join("catalog.toml").is_file() {
            bail!("no content at {} (catalog.toml missing)", root.display());
        }
        let lib = Library::load(root);
        if !lib.errors.is_empty() {
            let list: Vec<String> = lib.errors.iter().map(|e| e.to_string()).collect();
            bail!(
                "content has {} error(s):\n  {}",
                list.len(),
                list.join("\n  ")
            );
        }
        let version = content_hash(root)?;

        let mut store = ContentStore {
            lib,
            version,
            premium_tiers,
            catalog: Arc::new(empty_catalog()),
            catalog_json: Arc::default(),
            guide_json: Arc::default(),
            problems: BTreeMap::new(),
            slugs: BTreeSet::new(),
            runnable,
        };
        store.slugs = store.lib.catalog.iter().map(|c| c.slug.clone()).collect();
        let catalog = store.build_catalog();
        store.catalog_json = Arc::new(serde_json::to_vec(&catalog)?);
        store.catalog = Arc::new(catalog);
        store.guide_json = Arc::new(serde_json::to_vec(&store.build_guide())?);
        let mut problems = BTreeMap::new();
        for (slug, pack) in store.lib.packs() {
            problems.insert(slug.clone(), Arc::new(store.build_problem(pack)));
        }
        store.problems = problems;
        Ok(store)
    }

    pub fn catalog(&self) -> &dto::Catalog {
        &self.catalog
    }
    pub fn catalog_json(&self) -> Arc<Vec<u8>> {
        self.catalog_json.clone()
    }
    pub fn guide_json(&self) -> Arc<Vec<u8>> {
        self.guide_json.clone()
    }

    /// A slug the catalogue lists. Writes naming anything else are refused, so
    /// progress rows can only ever point at real problems.
    pub fn has(&self, slug: &str) -> bool {
        self.slugs.contains(slug)
    }

    pub fn item(&self, slug: &str) -> Option<&CatalogItem> {
        self.lib.item(slug)
    }

    pub fn pack(&self, slug: &str) -> Option<&ProblemPack> {
        self.lib.pack(slug)
    }

    pub fn language(&self, id: &str) -> Option<&LanguageDef> {
        self.lib.language(id)
    }

    pub fn is_premium(&self, tier: Tier) -> bool {
        self.premium_tiers.contains(&tier)
    }

    pub fn is_runnable(&self, lang: &str) -> bool {
        self.runnable.contains(lang)
    }

    /// The problem payload. `entitled` decides whether a premium problem comes
    /// with its sources or `locked`.
    pub fn problem(&self, slug: &str, entitled: bool) -> Option<Arc<dto::Problem>> {
        let full = self.problems.get(slug)?;
        if !full.premium || entitled {
            return Some(full.clone());
        }
        let mut locked = (**full).clone();
        locked.locked = true;
        locked.sources.clear();
        Some(Arc::new(locked))
    }

    pub fn default_input(&self, slug: &str) -> Option<&InputMap> {
        self.pack(slug).map(|p| &p.meta.default_input)
    }

    // ── builders ────────────────────────────────────────────────────────────

    fn build_catalog(&self) -> dto::Catalog {
        let lib = &self.lib;
        let languages: Vec<dto::Language> = lib
            .languages
            .iter()
            .map(|l| dto::Language {
                id: l.id.clone(),
                label: l.label.clone(),
                ext: l.ext.clone(),
                syntax: l.syntax.clone(),
                comment: l.comment.clone(),
                order: l.order,
                runnable: self.runnable.contains(&l.id),
            })
            .collect();

        let mut categories: Vec<dto::CatalogCategory> = lib
            .categories
            .iter()
            .map(|name| dto::CatalogCategory {
                name: name.clone(),
                problems: vec![],
            })
            .collect();
        for item in &lib.catalog {
            let langs = lib
                .pack(&item.slug)
                .map(|p| {
                    lib.languages
                        .iter()
                        .filter(|l| p.source(&l.id).is_some())
                        .map(|l| l.id.clone())
                        .collect()
                })
                .unwrap_or_default();
            let row = dto::CatalogProblem {
                slug: item.slug.clone(),
                title: item.title.clone(),
                category: item.category.clone(),
                difficulty: item.difficulty,
                tier: item.tier,
                leetcode_url: item.leetcode_url(),
                viz: item.viz,
                langs,
                premium: self.is_premium(item.tier),
            };
            if let Some(cat) = categories.iter_mut().find(|c| c.name == item.category) {
                cat.problems.push(row);
            }
        }

        let tiers = Tier::ALL
            .iter()
            .map(|t| {
                let (animated, count) = lib.tier_progress(*t);
                dto::TierInfo {
                    id: *t,
                    title: t.title(),
                    count,
                    animated,
                }
            })
            .collect();

        dto::Catalog {
            content_version: self.version.clone(),
            tiers,
            total: lib.catalog.len(),
            animated: lib.catalog.iter().filter(|c| c.viz).count(),
            categories,
            languages,
        }
    }

    fn build_guide(&self) -> dto::Guide {
        let g = &self.lib.guide;
        dto::Guide {
            topics: g
                .topics
                .iter()
                .map(|t| dto::GuideTopic {
                    id: t.id.clone(),
                    title: t.title.clone(),
                    kind: match t.kind {
                        dsa_content::TopicKind::Structure => "structure",
                        dsa_content::TopicKind::Technique => "technique",
                    },
                    emoji: t.emoji.clone(),
                    what: t.what.clone(),
                    complexity: t
                        .complexity
                        .iter()
                        .map(|r| dto::GuideOpRow {
                            op: r.op.clone(),
                            big: r.big.clone(),
                            note: r.note.clone(),
                        })
                        .collect(),
                    syntax: t.syntax.clone(),
                    notes: t.notes.clone(),
                })
                .collect(),
            cheatsheet: g
                .cheatsheet
                .iter()
                .map(|s| dto::CheatSection {
                    name: s.name.clone(),
                    rows: s
                        .rows
                        .iter()
                        .map(|r| dto::CheatRow {
                            topic: r.topic.clone(),
                            code: r.code.clone(),
                        })
                        .collect(),
                })
                .collect(),
            by_category: g.by_category.clone(),
        }
    }

    fn build_problem(&self, pack: &ProblemPack) -> dto::Problem {
        let lib = &self.lib;
        let meta = &pack.meta;
        let sources = lib
            .languages
            .iter()
            .filter_map(|l| {
                let parsed = pack.source(&l.id)?;
                let (harness, synthesized) = match harness_for(pack, &l.id) {
                    Some((h, synth)) => (Some(h), synth),
                    None => (None, false),
                };
                let starter = if parsed.clean.trim().is_empty() {
                    pack.practice.get(&l.id).cloned().unwrap_or_default()
                } else {
                    dsa_core::practice::starter(&parsed.clean, &l.id)
                };
                Some(dto::ProblemSource {
                    lang: l.id.clone(),
                    code: parsed.clean.clone(),
                    tag_lines: parsed.tag_to_line.clone(),
                    line_tags: parsed
                        .line_to_tag
                        .iter()
                        .map(|(line, tag)| (line.to_string(), tag.clone()))
                        .collect(),
                    starter,
                    harness,
                    synthesized,
                })
            })
            .collect();

        let tests = meta
            .tests
            .iter()
            .enumerate()
            .map(|(i, t)| dto::TestCaseView {
                name: if t.name.is_empty() {
                    format!("case {}", i + 1)
                } else {
                    t.name.clone()
                },
                input: t.input.clone(),
                stdin: dsa_harness::serialize_input(&meta.inputs, &t.input),
                expected: t.expected.clone(),
                edge: t.edge,
            })
            .collect();

        let related = meta
            .related
            .iter()
            .filter_map(|s| lib.item(s))
            .map(|c| dto::RelatedProblem {
                slug: c.slug.clone(),
                title: c.title.clone(),
                difficulty: c.difficulty,
            })
            .collect();

        let premium = self.is_premium(meta.tier);
        dto::Problem {
            slug: meta.slug.clone(),
            title: meta.title.clone(),
            category: meta.category.clone(),
            difficulty: meta.difficulty,
            tier: meta.tier,
            description: meta.description.clone(),
            approach: meta.approach.clone(),
            complexity: meta.complexity.clone(),
            leetcode_url: meta.leetcode_url(),
            inputs: meta.inputs.clone(),
            default_input: meta.default_input.clone(),
            default_fields: meta
                .inputs
                .iter()
                .map(|f| {
                    let text = meta
                        .default_input
                        .get(&f.name)
                        .map(|v| v.to_editable())
                        .unwrap_or_default();
                    (f.name.clone(), text)
                })
                .collect(),
            tests,
            hints: meta.hints.clone(),
            related,
            guide_topics: lib
                .guide
                .by_category
                .get(&meta.category)
                .cloned()
                .unwrap_or_default(),
            has_trace: pack.has_script(),
            premium,
            locked: false,
            sources,
        }
    }
}

/// The program a solution is spliced into — the desktop's `harness_for`: the
/// pack's hand-written `practice/<lang>.txt` if it ships one, otherwise one
/// derived from the input schema. `None` when neither is possible, in which
/// case "solution" mode runs the user's text as the whole program. The flag is
/// true for a synthesized harness.
pub fn harness_for(pack: &ProblemPack, lang: &str) -> Option<(String, bool)> {
    if let Some(shipped) = pack.practice.get(lang) {
        return Some((shipped.clone(), false));
    }
    let reference = pack.source(lang)?;
    dsa_core::synth::synthesize(lang, &pack.meta, &reference.clean).map(|h| (h, true))
}

/// What Run compiles — the desktop's `program`: a full program is used
/// verbatim; a solution is assembled into the harness fresh every time.
pub fn program(pack: &ProblemPack, lang: &str, code: &str, full_program: bool) -> String {
    if full_program {
        return code.to_string();
    }
    let reference = pack.source(lang).map(|p| p.clean.as_str()).unwrap_or("");
    let harness = harness_for(pack, lang).map(|(h, _)| h);
    dsa_core::practice::assemble(code, reference, harness.as_deref())
}

fn empty_catalog() -> dto::Catalog {
    dto::Catalog {
        content_version: String::new(),
        tiers: vec![],
        categories: vec![],
        languages: vec![],
        total: 0,
        animated: 0,
    }
}

/// SHA-256 over every file's relative path and bytes, in sorted order, so the
/// same tree always hashes the same on every machine and every task.
fn content_hash(root: &Path) -> Result<String> {
    let mut files: Vec<_> = walk(root)?;
    files.sort();
    let mut h = Sha256::new();
    for rel in files {
        h.update(rel.to_string_lossy().replace('\\', "/").as_bytes());
        h.update([0]);
        // Line endings differ between a Windows checkout and the Linux image;
        // normalise so the version is a property of the content, not the OS.
        let bytes = std::fs::read(root.join(&rel))?;
        let text: Vec<u8> = bytes.into_iter().filter(|b| *b != b'\r').collect();
        h.update(&text);
        h.update([0]);
    }
    Ok(hex::encode(&h.finalize()[..8]))
}

fn walk(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> ContentStore {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../content");
        ContentStore::load(
            &root,
            vec![Tier::Extra],
            ["go", "cpp", "java", "python"]
                .into_iter()
                .map(String::from)
                .collect(),
        )
        .expect("repo content loads")
    }

    #[test]
    fn the_real_catalogue_loads_whole() {
        let s = store();
        let c = s.catalog();
        assert_eq!(c.total, 287);
        assert_eq!(c.categories.len(), 22);
        assert_eq!(
            c.categories.iter().map(|c| c.problems.len()).sum::<usize>(),
            287
        );
        assert_eq!(c.tiers.len(), 4);
        // Extra widens to everything.
        assert_eq!(c.tiers.last().unwrap().count, 287);
        assert!(!s.version.is_empty());
    }

    #[test]
    fn problems_carry_sources_starters_and_stdin() {
        let s = store();
        let p = s.problem("two-sum", false).unwrap();
        assert!(!p.locked);
        assert_eq!(p.sources.len(), 3, "go, cpp, java");
        let go = p.sources.iter().find(|x| x.lang == "go").unwrap();
        assert!(!go.code.contains("//@"), "markers are stripped");
        assert!(go.tag_lines.contains_key("init"));
        assert!(go.starter.contains("write your code here"));
        assert!(go.harness.as_deref().unwrap().contains("func main()"));
        assert_eq!(p.tests[0].stdin, "2 7 11 15 3\n14\n");
        assert_eq!(p.default_fields["nums"], "2 7 11 15 3");
    }

    #[test]
    fn premium_problems_are_locked_unless_entitled() {
        let s = store();
        let premium = s
            .catalog()
            .categories
            .iter()
            .flat_map(|c| &c.problems)
            .find(|p| p.premium)
            .expect("extra tier is premium in this test")
            .slug
            .clone();
        let locked = s.problem(&premium, false).unwrap();
        assert!(locked.locked);
        assert!(locked.sources.is_empty());
        assert!(!locked.description.is_empty(), "the statement stays public");
        assert!(!s.problem(&premium, true).unwrap().locked);
    }
}
