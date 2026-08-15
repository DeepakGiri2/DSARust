//! The Practice starter and assembler, against the real content tree.
//!
//! The unit tests in `dsa_core::practice` pin the behaviour on hand-written
//! sources. This one asks the harder question: across all 287 packs and every
//! language they ship, does the starter actually remove the answer, and does
//! what comes back out of the assembler still look like a program?
//!
//! Skipped when there is no content root, so a checkout without one still runs
//! a green test suite.

use dsa_core::practice::{assemble, starter};

fn library() -> Option<dsa_content::Library> {
    let root = dsa_content::find_content_root(&[]).ok()?;
    Some(dsa_content::Library::load(root))
}

/// A line that only a solution body has: control flow, a returned value, a
/// mutation. Declarations deliberately survive — a design problem's struct and
/// a file-scope `const` are the shell you are asked to fill in, and blanking
/// them would leave the user a skeleton that cannot compile.
fn is_solution_logic(line: &str) -> bool {
    const STARTS: [&str; 8] = [
        "return ", "for ", "while ", "if ", "else", "switch ", "res.", "ans.",
    ];
    STARTS.iter().any(|s| line.starts_with(s)) || line.contains("++")
}

#[test]
fn no_pack_hands_the_answer_to_the_practice_editor() {
    let Some(lib) = library() else { return };
    let mut checked = 0;
    let mut blanked = 0;
    let mut leaked = Vec::new();

    for (slug, pack) in lib.packs() {
        for lang in &lib.languages {
            let Some(reference) = pack.source(&lang.id) else {
                continue;
            };
            checked += 1;
            let skeleton = starter(&reference.clean, &lang.id);
            if skeleton.contains("write your code here") {
                blanked += 1;
            }

            let survivors: Vec<&str> = reference
                .clean
                .lines()
                .map(str::trim)
                .filter(|l| l.len() > 10 && !l.ends_with('{') && is_solution_logic(l))
                .filter(|l| skeleton.contains(*l))
                .collect();
            if !survivors.is_empty() {
                leaked.push(format!("{slug} [{}]: {}", lang.id, survivors[0]));
            }
        }
    }

    assert!(
        checked > 200,
        "content root looked empty ({checked} sources)"
    );
    assert_eq!(
        blanked,
        checked,
        "{} sources produced no placeholder at all",
        checked - blanked
    );
    assert!(
        leaked.is_empty(),
        "{} sources leak solution logic into the starter, e.g.\n  {}",
        leaked.len(),
        leaked
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_harness_still_assembles_around_the_starter() {
    let Some(lib) = library() else { return };
    let mut with_harness = 0;
    let mut broken = Vec::new();

    for (slug, pack) in lib.packs() {
        for lang in &lib.languages {
            let (Some(reference), Some(harness)) =
                (pack.source(&lang.id), pack.practice.get(&lang.id))
            else {
                continue;
            };
            with_harness += 1;

            let skeleton = starter(&reference.clean, &lang.id);
            let program = assemble(&skeleton, &reference.clean, Some(harness));

            // The splice found the solution: the placeholder is in, the
            // reference body is out, and the harness's entry point survived.
            let entry = match lang.id.as_str() {
                "go" => "func main(",
                "java" => "static void main(",
                _ => "int main(",
            };
            if !program.contains("write your code here") {
                broken.push(format!("{slug} [{}]: starter not spliced in", lang.id));
            } else if !program.contains(entry) {
                broken.push(format!("{slug} [{}]: lost {entry}", lang.id));
            }
        }
    }

    assert!(with_harness > 200, "expected the packs that ship harnesses");
    assert!(
        broken.is_empty(),
        "{} harnesses no longer assemble, e.g.\n  {}",
        broken.len(),
        broken
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
