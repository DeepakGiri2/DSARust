//! Content linting — the safety net that lets 250 problems live as data.
//!
//! Without a compiler watching over the content tree, the failure mode is
//! silent rot: a renamed `//@tag` that stops highlighting, a default input the
//! script can no longer trace, a test that no longer matches. The linter runs
//! every pack the way the app would (including actually executing the trace on
//! the default input) and reports what a reviewer would otherwise have to
//! click through 250 problems to find.

use crate::library::Library;
use dsa_core::code::missing_tags;
use dsa_core::problem::{validate_inputs, Tier};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Breaks the experience: the problem will not trace, or highlights wrong.
    Error,
    /// Works, but something is missing or inconsistent.
    Warning,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub slug: String,
    pub severity: Severity,
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warn ",
        };
        write!(f, "{tag}  {:<44} {}", self.slug, self.message)
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub issues: Vec<Issue>,
    pub packs_checked: usize,
    pub traces_run: usize,
    pub total_steps: usize,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }
    pub fn warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
    pub fn ok(&self) -> bool {
        self.errors() == 0
    }
}

/// Lint the whole library. `deep` also runs every trace on its default input,
/// which is slower but is the only check that proves a problem still animates.
pub fn lint(lib: &Library, deep: bool) -> Report {
    let mut report = Report::default();
    let mut push = |slug: &str, severity, message: String| {
        report.issues.push(Issue {
            slug: slug.to_string(),
            severity,
            message,
        });
    };

    // Load-time failures are errors in their own right.
    for e in &lib.errors {
        push(&e.slug, Severity::Error, e.message.clone());
    }

    let catalog_slugs: BTreeSet<&str> = lib.catalog.iter().map(|c| c.slug.as_str()).collect();

    for (slug, pack) in lib.packs() {
        report.packs_checked += 1;

        for w in &pack.warnings {
            push(slug, Severity::Warning, w.clone());
        }

        // ── metadata agrees with the catalog ────────────────────────────────
        match lib.item(slug) {
            Some(item) => {
                if item.title != pack.meta.title {
                    push(
                        slug,
                        Severity::Warning,
                        format!(
                            "title differs from catalog (\"{}\" vs \"{}\")",
                            pack.meta.title, item.title
                        ),
                    );
                }
                if item.category != pack.meta.category {
                    push(
                        slug,
                        Severity::Warning,
                        format!(
                            "category differs from catalog (\"{}\" vs \"{}\")",
                            pack.meta.category, item.category
                        ),
                    );
                }
                if item.difficulty != pack.meta.difficulty {
                    push(
                        slug,
                        Severity::Warning,
                        "difficulty differs from catalog".into(),
                    );
                }
            }
            None => push(slug, Severity::Error, "not present in catalog.toml".into()),
        }

        if pack.meta.description.trim().is_empty() {
            push(slug, Severity::Warning, "no description".into());
        }
        if pack.meta.approach.trim().is_empty() {
            push(slug, Severity::Warning, "no approach summary".into());
        }
        if pack.meta.complexity.trim().is_empty() {
            push(slug, Severity::Warning, "no complexity line".into());
        }

        // ── inputs ─────────────────────────────────────────────────────────
        if pack.meta.inputs.is_empty() && pack.has_script() {
            push(slug, Severity::Warning, "no input fields declared".into());
        }
        let declared: BTreeSet<&str> = pack.meta.inputs.iter().map(|f| f.name.as_str()).collect();
        for key in pack.meta.default_input.keys() {
            if !declared.contains(key.as_str()) {
                push(
                    slug,
                    Severity::Warning,
                    format!("default_input has \"{key}\" with no matching [[inputs]] field"),
                );
            }
        }
        for e in validate_inputs(&pack.meta.inputs, &pack.meta.default_input) {
            push(
                slug,
                Severity::Error,
                format!("default input rejected: {e}"),
            );
        }
        if let Some(e) = lib.validate(slug, &pack.meta.default_input) {
            push(
                slug,
                Severity::Error,
                format!("default input rejected by validate(): {e}"),
            );
        }

        // ── sources and tags ───────────────────────────────────────────────
        if pack.sources.is_empty() {
            push(slug, Severity::Error, "no solution sources".into());
        }
        let used = crate::script::ScriptHost::declared_tags(&pack.script_source);
        if pack.has_script() && used.is_empty() {
            push(
                slug,
                Severity::Warning,
                "script records no tagged steps".into(),
            );
        }
        for (lang, parsed) in &pack.sources {
            let miss = missing_tags(parsed, used.iter().map(|s| s.as_str()));
            if !miss.is_empty() {
                push(
                    slug,
                    Severity::Error,
                    format!("{lang} source is missing tag(s): {}", miss.join(", ")),
                );
            }
            let declared_tags: BTreeSet<&str> =
                parsed.tag_to_line.keys().map(|s| s.as_str()).collect();
            let unused: Vec<&str> = declared_tags
                .iter()
                .copied()
                .filter(|t| !used.iter().any(|u| u == t))
                .collect();
            if !unused.is_empty() && pack.has_script() {
                push(
                    slug,
                    Severity::Warning,
                    format!("{lang} source has unused tag(s): {}", unused.join(", ")),
                );
            }
        }

        // ── tests ──────────────────────────────────────────────────────────
        if pack.meta.tests.is_empty() {
            push(slug, Severity::Warning, "no test cases".into());
        }
        for (i, t) in pack.meta.tests.iter().enumerate() {
            if t.expected.trim().is_empty() {
                push(
                    slug,
                    Severity::Warning,
                    format!("test #{} has no expected output", i + 1),
                );
            }
            for e in validate_inputs(&pack.meta.inputs, &t.input) {
                push(
                    slug,
                    Severity::Error,
                    format!("test #{} input rejected: {e}", i + 1),
                );
            }
        }

        for r in &pack.meta.related {
            if !catalog_slugs.contains(r.as_str()) {
                push(
                    slug,
                    Severity::Warning,
                    format!("related problem \"{r}\" is not in the catalog"),
                );
            }
        }

        // ── the trace actually runs ────────────────────────────────────────
        if deep && pack.has_script() {
            match lib.trace(slug, &pack.meta.default_input) {
                Ok(trace) => {
                    report.traces_run += 1;
                    report.total_steps += trace.len();
                    if trace.is_empty() {
                        push(slug, Severity::Error, "trace recorded zero steps".into());
                    } else if trace.len() < 3 {
                        push(
                            slug,
                            Severity::Warning,
                            format!("trace is only {} step(s) long", trace.len()),
                        );
                    }
                    // A tag the sources never declare means the code panel
                    // silently keeps the previous highlight.
                    let step_tags: BTreeSet<&str> =
                        trace.steps.iter().map(|s| s.tag.as_str()).collect();
                    for (lang, parsed) in &pack.sources {
                        let miss = missing_tags(parsed, step_tags.iter().copied());
                        if !miss.is_empty() {
                            push(
                                slug,
                                Severity::Error,
                                format!(
                                    "{lang}: trace emits tag(s) the source does not mark: {}",
                                    miss.join(", ")
                                ),
                            );
                        }
                    }
                    if trace.steps.iter().any(|s| s.views.is_empty()) {
                        push(slug, Severity::Warning, "some steps render no views".into());
                    }
                    if trace.steps.iter().any(|s| s.note.trim().is_empty()) {
                        push(
                            slug,
                            Severity::Warning,
                            "some steps have no explanation".into(),
                        );
                    }
                    if trace.steps.iter().any(|s| s.frames.is_empty()) {
                        push(
                            slug,
                            Severity::Warning,
                            "some steps have an empty call stack (missing push?)".into(),
                        );
                    }
                }
                Err(e) => push(slug, Severity::Error, format!("trace failed: {e}")),
            }
        }
    }

    // ── catalog-level coverage ─────────────────────────────────────────────
    for tier in Tier::ALL {
        let (viz, total) = lib.tier_progress(tier);
        if total == 0 {
            report.issues.push(Issue {
                slug: format!("catalog:{}", tier.label()),
                severity: Severity::Warning,
                message: "tier has no problems".into(),
            });
        } else {
            log::debug!("tier {}: {viz}/{total} animated", tier.label());
        }
    }

    report
        .issues
        .sort_by(|a, b| a.severity.cmp(&b.severity).then(a.slug.cmp(&b.slug)));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str, trace: &str, go: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsa-lint-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("problems/two-sum/code")).unwrap();
        std::fs::write(
            dir.join("languages.toml"),
            "[[language]]\nid = \"go\"\nlabel = \"Go\"\next = \"go\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("catalog.toml"),
            "[[category]]\nname = \"A\"\nproblems = [ { title = \"Two Sum\", difficulty = \"Easy\", tier = \"50\" } ]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("problems/two-sum/problem.toml"),
            r#"slug = "two-sum"
title = "Two Sum"
category = "A"
difficulty = "Easy"
tier = "50"
description = "d"
approach = "a"
complexity = "O(n)"

[[inputs]]
name = "nums"
type = "int-array"

[default_input]
nums = [1, 2]

[[tests]]
expected = "0 1"
[tests.input]
nums = [1, 2]
"#,
        )
        .unwrap();
        std::fs::write(dir.join("problems/two-sum/code/go.txt"), go).unwrap();
        std::fs::write(dir.join("problems/two-sum/trace.rhai"), trace).unwrap();
        dir
    }

    const GOOD_TRACE: &str = r#"fn trace(input) {
        enter("twoSum");
        step("init", "start", [ array("nums", input.nums) ]);
        step("loop", "scan", [ array("nums", input.nums).ptr("i", 0) ]);
        step("done", "end", [ array("nums", input.nums) ]);
    }"#;
    const GOOD_GO: &str = "func twoSum() { //@init\n  for {} //@loop\n  return //@done\n}\n";

    #[test]
    fn a_healthy_pack_lints_clean() {
        let lib = Library::load(fixture("clean", GOOD_TRACE, GOOD_GO));
        let r = lint(&lib, true);
        assert!(r.ok(), "{:#?}", r.issues);
        assert_eq!(r.traces_run, 1);
        assert_eq!(r.total_steps, 3);
    }

    #[test]
    fn a_tag_missing_from_a_source_is_an_error() {
        let lib = Library::load(fixture(
            "missingtag",
            GOOD_TRACE,
            "func twoSum() { //@init\n}\n",
        ));
        let r = lint(&lib, true);
        assert!(!r.ok());
        assert!(r
            .issues
            .iter()
            .any(|i| i.message.contains("loop") && i.severity == Severity::Error));
    }

    #[test]
    fn an_unused_source_tag_is_only_a_warning() {
        let go = "func f() { //@init\n //@loop\n //@done\n //@spare\n}";
        let lib = Library::load(fixture("unused", GOOD_TRACE, go));
        let r = lint(&lib, true);
        assert!(
            r.ok(),
            "unused tags must not fail the build: {:#?}",
            r.issues
        );
        assert!(r.issues.iter().any(|i| i.message.contains("unused tag")));
    }

    #[test]
    fn a_failing_trace_is_reported_with_its_message() {
        let lib = Library::load(fixture(
            "failtrace",
            "fn trace(input) { push(\"f\"); nonexistent_call(); }",
            GOOD_GO,
        ));
        let r = lint(&lib, true);
        assert!(r.issues.iter().any(|i| i.message.contains("trace failed")));
    }

    #[test]
    fn a_default_input_violating_its_own_constraints_is_caught() {
        let dir = fixture("badinput", GOOD_TRACE, GOOD_GO);
        let p = dir.join("problems/two-sum/problem.toml");
        let text = std::fs::read_to_string(&p).unwrap().replace(
            "name = \"nums\"\ntype = \"int-array\"",
            "name = \"nums\"\ntype = \"int-array\"\nmax_len = 1",
        );
        std::fs::write(&p, text).unwrap();
        let lib = Library::load(&dir);
        let r = lint(&lib, false);
        assert!(r
            .issues
            .iter()
            .any(|i| i.message.contains("default input rejected")));
    }

    #[test]
    fn steps_without_explanations_are_flagged() {
        let trace = r#"fn trace(input) {
            enter("f");
            step("init", "", [ array("nums", input.nums) ]);
            step("loop", "", []);
            step("done", "", []);
        }"#;
        let lib = Library::load(fixture("noexpl", trace, GOOD_GO));
        let r = lint(&lib, true);
        assert!(r
            .issues
            .iter()
            .any(|i| i.message.contains("no explanation")));
        assert!(r
            .issues
            .iter()
            .any(|i| i.message.contains("render no views")));
    }
}
