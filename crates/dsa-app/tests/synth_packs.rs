//! Does a generated harness actually produce the right answers?
//!
//! `dsa_core::synth` decides, from a pack's input schema and its entry
//! signature, that it can build a whole program. That decision is only worth
//! anything if the program agrees with the pack's own test cases — a harness
//! that compiles and prints in the wrong format is worse than no harness,
//! because it fails the user's correct code and blames them for it.
//!
//! So this compiles every generated harness around the pack's *reference*
//! solution and runs the pack's declared tests through it. The reference is
//! known-good, so any disagreement is the harness's fault, and this is the
//! thing that would catch it.
//!
//! Go only: it is the one toolchain that can be counted on here, and the
//! generator's per-language differences are covered by unit tests in
//! `dsa_core::synth`. Skipped when Go is not installed.

use dsa_core::synth::synthesize;
use dsa_harness::{run_tests, Backend, Harness};

const LANG: &str = "go";

#[test]
fn every_generated_harness_reproduces_its_packs_expected_answers() {
    let Ok(root) = dsa_content::find_content_root(&[]) else {
        return;
    };
    let lib = dsa_content::Library::load(root);
    let harness = Harness::detect(&lib.languages);
    if harness.backend(LANG) != Backend::Local {
        eprintln!("skipped: no local Go toolchain");
        return;
    }

    let mut generated = 0;
    let mut skipped = 0;
    let mut wrong: Vec<String> = Vec::new();
    // Packs whose Go source calls something it never defines — a `MaxHeap`, a
    // `maxi` helper. Those cannot compile on their own whatever surrounds them,
    // so they are the content's problem and not the harness's. A user writing
    // their own self-contained solution still runs fine.
    let mut incomplete: Vec<String> = Vec::new();

    for (slug, pack) in lib.packs() {
        // Only the packs that ship no harness of their own — the others are
        // covered by `practice_end_to_end`.
        if pack.practice.contains_key(LANG) || pack.meta.tests.is_empty() {
            continue;
        }
        let Some(reference) = pack.source(LANG) else {
            continue;
        };
        let Some(program) = synthesize(LANG, &pack.meta, &reference.clean) else {
            skipped += 1;
            continue;
        };
        generated += 1;

        let outcomes = run_tests(
            &harness,
            LANG,
            &program,
            &pack.meta.inputs,
            &pack.meta.tests,
        );
        for (i, o) in outcomes.iter().enumerate() {
            if o.status.passed() {
                continue;
            }
            // "undefined: MaxHeap" is the reference missing its own helper;
            // "undefined: sort" would be a missing import, and that *is* the
            // generator's job, so it stays a failure.
            let missing = o
                .detail
                .split("undefined: ")
                .skip(1)
                .filter_map(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                .collect::<Vec<_>>();
            if !missing.is_empty() && missing.iter().all(|m| !is_std_package(m)) {
                incomplete.push(format!("{slug} (needs {})", missing.join(", ")));
            } else {
                wrong.push(format!(
                    "{slug} case {}: {} — want {:?}, got {:?} {}",
                    i + 1,
                    o.status.label(),
                    o.expected,
                    o.actual,
                    o.detail.lines().take(4).collect::<Vec<_>>().join(" ¶ ")
                ));
            }
            break;
        }
    }

    eprintln!("generated {generated} harness(es), declined {skipped}");
    eprintln!(
        "{} pack(s) whose Go source is not self-contained: {}",
        incomplete.len(),
        incomplete.join(", ")
    );
    assert!(
        generated > 50,
        "the generator covered almost nothing ({generated}) — has the schema changed?"
    );
    assert!(
        wrong.is_empty(),
        "{} generated harness(es) disagree with their pack:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

fn is_std_package(name: &str) -> bool {
    matches!(
        name,
        "sort"
            | "math"
            | "bytes"
            | "errors"
            | "unicode"
            | "heap"
            | "list"
            | "bits"
            | "utf8"
            | "strings"
            | "strconv"
            | "fmt"
            | "os"
            | "bufio"
    )
}
