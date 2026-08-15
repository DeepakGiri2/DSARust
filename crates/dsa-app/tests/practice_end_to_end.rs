//! The Practice tab's whole path, for real: take the pack's starter, write a
//! solution into it the way a user would, assemble it against the pack's
//! harness, and hand the result to an actual compiler.
//!
//! The unit tests prove the text lands in the right place. Only this proves the
//! thing that comes out is a program — the port shipped an `assemble` that
//! returned its input, so "solution" view produced a bare function with no
//! `main` and every Run was a compile error.
//!
//! Every case is skipped unless there is a content root and that language's
//! toolchain is installed locally, so this never turns a build into a network
//! or install requirement. Java earns its place next to Go: its harness holds
//! the solution inside `class Main`, so it is the one that proves the splice
//! restores indentation rather than merely finding the right lines.

use dsa_core::practice::{assemble, starter};
use dsa_harness::{run_tests, Backend, Harness};

/// What someone would type into the Two Sum starter, per language.
const SOLUTIONS: [(&str, &str); 2] = [
    (
        "go",
        "\
func twoSum(nums []int, target int) []int {
    seen := map[int]int{}
    for i, x := range nums {
        if j, ok := seen[target-x]; ok {
            return []int{j, i}
        }
        seen[x] = i
    }
    return nil
}
",
    ),
    (
        "java",
        "\
static int[] twoSum(int[] nums, int target) {
    Map<Integer, Integer> seen = new HashMap<>();
    for (int i = 0; i < nums.length; i++) {
        if (seen.containsKey(target - nums[i])) {
            return new int[]{seen.get(target - nums[i]), i};
        }
        seen.put(nums[i], i);
    }
    return new int[0];
}
",
    ),
];

struct Fixture {
    lib: dsa_content::Library,
    harness: Harness,
}

fn fixture(lang: &str) -> Option<Fixture> {
    let root = dsa_content::find_content_root(&[]).ok()?;
    let lib = dsa_content::Library::load(root);
    let harness = Harness::detect(&lib.languages);
    // Remote would work too, but a test that needs the network is a test that
    // fails on a train.
    if harness.backend(lang) != Backend::Local {
        eprintln!("skipped {lang}: no local toolchain");
        return None;
    }

    // Being on PATH is not the same as working — Windows ships a `javac` shim
    // that survives a deleted JDK and then fails silently. Compile the pack's
    // own harness first: if that does not build, nothing this test assembles
    // would either, and the failure would be about the machine rather than
    // about the code under test.
    let canary = lib
        .pack("two-sum")
        .and_then(|p| p.practice.get(lang))
        .cloned()?;
    let probe = harness.run(lang, &canary, "2 7 11\n9\n");
    if !probe.compiled {
        eprintln!(
            "skipped {lang}: the local toolchain cannot build a known-good program ({})",
            probe.error.unwrap_or(probe.compile_output)
        );
        return None;
    }

    Some(Fixture { lib, harness })
}

#[test]
fn a_solution_written_into_the_starter_compiles_and_passes_the_packs_tests() {
    let mut ran = 0;
    for (lang, mine) in SOLUTIONS {
        let Some(Fixture { lib, harness }) = fixture(lang) else {
            continue;
        };
        ran += 1;
        let pack = lib
            .pack("two-sum")
            .expect("two-sum is in every content root");
        let reference = &pack.source(lang).expect("two-sum ships every lang").clean;

        let program = assemble(mine, reference, pack.practice.get(lang).map(String::as_str));
        assert!(
            program.contains("main("),
            "[{lang}] assembled program has no entry point:\n{program}"
        );

        let outcomes = run_tests(
            &harness,
            lang,
            &program,
            &pack.meta.inputs,
            &pack.meta.tests,
        );
        assert!(!outcomes.is_empty(), "two-sum ships test cases");
        for (i, o) in outcomes.iter().enumerate() {
            assert!(
                o.status.passed(),
                "[{lang}] case {} came back {}: want {:?}, got {:?}\n{}",
                i + 1,
                o.status.label(),
                o.expected,
                o.actual,
                o.detail
            );
        }
    }
    eprintln!("ran against {ran} local toolchain(s)");
}

#[test]
fn the_untouched_starter_builds_but_does_not_answer_anything() {
    for (lang, _) in SOLUTIONS {
        let Some(Fixture { lib, harness }) = fixture(lang) else {
            continue;
        };
        let pack = lib
            .pack("two-sum")
            .expect("two-sum is in every content root");
        let reference = &pack.source(lang).expect("two-sum ships every lang").clean;

        let skeleton = starter(reference, lang);
        let program = assemble(
            &skeleton,
            reference,
            pack.practice.get(lang).map(String::as_str),
        );
        let run = harness.run(lang, &program, "2 7 11\n9\n");

        // Opening the tab and pressing Run must reach the placeholder, not a
        // compile error: a skeleton that does not build cannot tell the user
        // whether their own code is what is wrong.
        assert!(
            run.compiled,
            "[{lang}] the empty starter does not build:\n{}",
            run.compile_output
        );
        assert!(
            run.stderr.contains("todo") || run.stdout.contains("todo"),
            "[{lang}] expected the placeholder to fire, got stdout {:?} / stderr {:?}",
            run.stdout,
            run.stderr
        );
    }
}
