//! A line diff, for showing what the AI changed.
//!
//! The point of the Fix mode is not to hand over corrected code — it is to
//! show you the line you got wrong. A replaced buffer cannot do that: you get
//! working code and no idea which part was the mistake. So the proposal is
//! shown against what you wrote, removals in red and additions in green, and
//! each change group can be taken or left on its own.
//!
//! Classic LCS, which is O(n·m) in time and memory. Solutions are tens of lines
//! and the cap below keeps a pathological input from eating the frame; beyond
//! it the answer degrades to "all of this became all of that", which is both
//! true and cheap.

/// Above this many cells the DP table is not worth building.
const MAX_CELLS: usize = 4_000_000;

/// Untouched lines that end a change group.
///
/// One, so a group is exactly one unbroken run of red and green — which is what
/// the eye reads as "a change" and therefore what a single tick should govern.
/// Merging edits that are two or three lines apart is right for a patch file,
/// where the unit is a hunk to apply; here the unit is a mistake to notice, and
/// two mistakes should be two decisions.
const GAP: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Same,
    Removed,
    Added,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub change: Change,
    pub text: String,
    /// 1-based line number in what the user wrote (`Same` and `Removed`).
    pub old_no: Option<usize>,
    /// 1-based line number in the proposal (`Same` and `Added`).
    pub new_no: Option<usize>,
    /// Which change group this belongs to; `None` for untouched context.
    pub hunk: Option<usize>,
}

/// A row in the rendered diff: a line, or a run of context that was folded away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Line(usize),
    /// How many unchanged lines were skipped here.
    Folded(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub lines: Vec<Line>,
    /// Number of change groups — what the per-hunk toggles count.
    pub hunks: usize,
    pub removed: usize,
    pub added: usize,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.hunks == 0
    }

    /// The text you get by taking the proposal for the accepted groups and
    /// keeping your own code everywhere else.
    ///
    /// A group not named in `accepted` counts as accepted, so the common call —
    /// `apply(&vec![true; hunks])` — and a short slice both mean "take it all".
    pub fn apply(&self, accepted: &[bool]) -> String {
        let taken = |h: Option<usize>| h.is_none_or(|i| accepted.get(i).copied().unwrap_or(true));
        let mut out = String::new();
        for line in &self.lines {
            let keep = match line.change {
                Change::Same => true,
                Change::Added => taken(line.hunk),
                Change::Removed => !taken(line.hunk),
            };
            if keep {
                out.push_str(&line.text);
                out.push('\n');
            }
        }
        out
    }

    /// Which rows to draw: every changed line, `context` untouched lines around
    /// each, and a fold marker standing in for the rest.
    ///
    /// Without this a two-line fix inside a forty-line function is two red rows
    /// lost in a wall of grey, and the reader has to hunt for the thing the
    /// screen exists to point at.
    pub fn rows(&self, context: usize) -> Vec<Row> {
        let near: Vec<bool> = (0..self.lines.len())
            .map(|i| {
                let lo = i.saturating_sub(context);
                let hi = (i + context).min(self.lines.len().saturating_sub(1));
                (lo..=hi).any(|j| self.lines[j].change != Change::Same)
            })
            .collect();

        let mut rows = Vec::new();
        let mut folded = 0;
        for (i, show) in near.iter().enumerate() {
            if *show {
                if folded > 0 {
                    rows.push(Row::Folded(folded));
                    folded = 0;
                }
                rows.push(Row::Line(i));
            } else {
                folded += 1;
            }
        }
        if folded > 0 {
            rows.push(Row::Folded(folded));
        }
        rows
    }
}

/// Diff `old` against `new`, line by line.
pub fn diff(old: &str, new: &str) -> Diff {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();

    let mut lines = if a.len().saturating_mul(b.len()) > MAX_CELLS {
        wholesale(&a, &b)
    } else {
        walk(&a, &b, &lcs_table(&a, &b))
    };

    // Group changes: a run of untouched lines shorter than `GAP` is a gap
    // inside one edit, not the space between two.
    let mut hunks = 0;
    let mut since = usize::MAX;
    for line in &mut lines {
        if line.change == Change::Same {
            since = since.saturating_add(1);
            continue;
        }
        if since >= GAP {
            hunks += 1;
        }
        line.hunk = Some(hunks - 1);
        since = 0;
    }

    Diff {
        removed: lines.iter().filter(|l| l.change == Change::Removed).count(),
        added: lines.iter().filter(|l| l.change == Change::Added).count(),
        hunks,
        lines,
    }
}

/// `table[i][j]` = length of the longest common subsequence of `a[i..]`, `b[j..]`.
fn lcs_table(a: &[&str], b: &[&str]) -> Vec<Vec<u32>> {
    let mut t = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            t[i][j] = if a[i] == b[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    t
}

fn walk(a: &[&str], b: &[&str], t: &[Vec<u32>]) -> Vec<Line> {
    let mut out = Vec::with_capacity(a.len().max(b.len()));
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            out.push(line(Change::Same, a[i], Some(i + 1), Some(j + 1)));
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            out.push(line(Change::Removed, a[i], Some(i + 1), None));
            i += 1;
        } else {
            out.push(line(Change::Added, b[j], None, Some(j + 1)));
            j += 1;
        }
    }
    // Whatever is left is a pure deletion or a pure insertion. Removals first,
    // so a replaced tail reads "this became that" rather than the reverse.
    for (k, text) in a[i..].iter().enumerate() {
        out.push(line(Change::Removed, text, Some(i + k + 1), None));
    }
    for (k, text) in b[j..].iter().enumerate() {
        out.push(line(Change::Added, text, None, Some(j + k + 1)));
    }
    out
}

/// The fallback for inputs too large to align: everything out, everything in.
fn wholesale(a: &[&str], b: &[&str]) -> Vec<Line> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    for (i, text) in a.iter().enumerate() {
        out.push(line(Change::Removed, text, Some(i + 1), None));
    }
    for (j, text) in b.iter().enumerate() {
        out.push(line(Change::Added, text, None, Some(j + 1)));
    }
    out
}

fn line(change: Change, text: &str, old_no: Option<usize>, new_no: Option<usize>) -> Line {
    Line {
        change,
        text: text.to_string(),
        old_no,
        new_no,
        hunk: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(d: &Diff) -> Vec<(Change, &str)> {
        d.lines
            .iter()
            .map(|l| (l.change, l.text.as_str()))
            .collect()
    }

    #[test]
    fn identical_text_has_nothing_to_review() {
        let src = "func f() int {\n    return 1\n}\n";
        let d = diff(src, src);
        assert!(d.is_empty());
        assert_eq!(d.removed, 0);
        assert_eq!(d.added, 0);
        assert!(d.lines.iter().all(|l| l.change == Change::Same));
    }

    #[test]
    fn a_changed_line_shows_as_a_removal_and_an_addition() {
        let mine = "func f() int {\n    return 1\n}\n";
        let theirs = "func f() int {\n    return 2\n}\n";
        let d = diff(mine, theirs);
        assert_eq!(
            kinds(&d),
            vec![
                (Change::Same, "func f() int {"),
                (Change::Removed, "    return 1"),
                (Change::Added, "    return 2"),
                (Change::Same, "}"),
            ]
        );
        assert_eq!((d.removed, d.added, d.hunks), (1, 1, 1));
    }

    #[test]
    fn line_numbers_follow_each_side_independently() {
        let d = diff("a\nb\nc\n", "a\nx\ny\nc\n");
        let by_change = |c: Change| -> Vec<(Option<usize>, Option<usize>)> {
            d.lines
                .iter()
                .filter(|l| l.change == c)
                .map(|l| (l.old_no, l.new_no))
                .collect()
        };
        assert_eq!(by_change(Change::Removed), vec![(Some(2), None)]);
        assert_eq!(
            by_change(Change::Added),
            vec![(None, Some(2)), (None, Some(3))]
        );
        // The trailing "c" is line 3 on the left and line 4 on the right.
        let last = d.lines.last().unwrap();
        assert_eq!((last.old_no, last.new_no), (Some(3), Some(4)));
    }

    #[test]
    fn an_insertion_removes_nothing() {
        let d = diff("a\nc\n", "a\nb\nc\n");
        assert_eq!(d.removed, 0);
        assert_eq!(d.added, 1);
        assert_eq!(d.hunks, 1);
    }

    #[test]
    fn each_unbroken_run_of_changes_is_its_own_group() {
        // Two edits with an untouched line between them are two mistakes, and
        // get a tick each.
        let split = diff("a\nX\nb\nY\nc\n", "a\n1\nb\n2\nc\n");
        assert_eq!(split.hunks, 2);

        // Adjacent edits are one run, and one decision — a replaced line is a
        // removal and an addition, not two changes.
        let together = diff("a\nX\nY\nb\n", "a\n1\n2\nb\n");
        assert_eq!(together.hunks, 1);
        assert_eq!((together.removed, together.added), (2, 2));
    }

    #[test]
    fn every_changed_line_belongs_to_exactly_one_group() {
        let d = diff("a\nX\nb\nY\nZ\nc\n", "a\n1\nb\n2\n3\nc\n");
        for l in &d.lines {
            match l.change {
                Change::Same => assert_eq!(l.hunk, None, "context is in no group"),
                _ => {
                    let h = l.hunk.expect("a change is always in a group");
                    assert!(h < d.hunks, "group index out of range");
                }
            }
        }
    }

    #[test]
    fn applying_everything_reproduces_the_proposal() {
        let mine = "func f() int {\n    x := 1\n    return x\n}\n";
        let theirs = "func f() int {\n    x := 2\n    y := 3\n    return x + y\n}\n";
        let d = diff(mine, theirs);
        assert_eq!(d.apply(&vec![true; d.hunks]), theirs);
    }

    #[test]
    fn applying_nothing_gives_back_exactly_what_the_user_wrote() {
        let mine = "func f() int {\n    x := 1\n    return x\n}\n";
        let theirs = "func f() int {\n    return 99\n}\n";
        let d = diff(mine, theirs);
        assert_eq!(d.apply(&vec![false; d.hunks]), mine);
    }

    #[test]
    fn a_single_group_can_be_taken_while_another_is_left() {
        let mine = "a\nWRONG1\nb\nc\nd\ne\nf\nWRONG2\ng\n";
        let theirs = "a\nRIGHT1\nb\nc\nd\ne\nf\nRIGHT2\ng\n";
        let d = diff(mine, theirs);
        assert_eq!(d.hunks, 2, "two runs, two decisions");

        let first_only = d.apply(&[true, false]);
        assert!(first_only.contains("RIGHT1"));
        assert!(first_only.contains("WRONG2"));
        assert!(!first_only.contains("WRONG1"));

        let second_only = d.apply(&[false, true]);
        assert!(second_only.contains("WRONG1"));
        assert!(second_only.contains("RIGHT2"));
    }

    #[test]
    fn an_empty_accepted_slice_means_take_it_all() {
        // The banner's "apply" builds its slice from the toggles; a stale or
        // short one must not silently drop the fix.
        let d = diff("a\n", "b\n");
        assert_eq!(d.apply(&[]), "b\n");
    }

    #[test]
    fn writing_from_scratch_is_all_additions() {
        let d = diff("", "func f() {}\n");
        assert_eq!(d.removed, 0);
        assert_eq!(d.added, 1);
        assert_eq!(d.apply(&[true]), "func f() {}\n");
    }

    #[test]
    fn folded_context_covers_every_line_exactly_once() {
        let mine = (1..=30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let theirs = mine.replace("line 15", "CHANGED");
        let d = diff(&mine, &theirs);

        let rows = d.rows(3);
        let shown: Vec<usize> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Line(i) => Some(*i),
                Row::Folded(_) => None,
            })
            .collect();
        let hidden: usize = rows
            .iter()
            .map(|r| match r {
                Row::Folded(n) => *n,
                Row::Line(_) => 0,
            })
            .sum();
        assert_eq!(shown.len() + hidden, d.lines.len(), "no line is lost");

        // Every changed line survives the fold, and the wall of context does not.
        for (i, l) in d.lines.iter().enumerate() {
            if l.change != Change::Same {
                assert!(shown.contains(&i), "hid a change at {i}");
            }
        }
        assert!(
            hidden > 15,
            "a 30-line file with one edit folds most of itself"
        );
    }

    #[test]
    fn nothing_is_folded_when_everything_is_near_a_change() {
        let d = diff("a\nb\n", "x\ny\n");
        assert!(d.rows(3).iter().all(|r| matches!(r, Row::Line(_))));
    }

    #[test]
    fn a_pathological_input_degrades_instead_of_hanging() {
        // Past the cell cap the answer is "all of this became all of that",
        // which still applies correctly — it just stops being minimal.
        let a: String = (0..3000).map(|i| format!("a{i}\n")).collect();
        let b: String = (0..3000).map(|i| format!("b{i}\n")).collect();
        let d = diff(&a, &b);
        assert_eq!(d.removed, 3000);
        assert_eq!(d.added, 3000);
        assert_eq!(d.apply(&[true]), b);
        assert_eq!(d.apply(&[false]), a);
    }
}
