//! Running a problem's test cases against a submission.
//!
//! The comparison is deliberately forgiving about whitespace and nothing else:
//! trailing newlines and platform line endings differ between toolchains and
//! are not the student's mistake, but a wrong number is a wrong number.

use crate::{serialize_input, Harness, RunOutcome};
use dsa_core::problem::{InputField, TestCase};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    CompileError,
    Crashed,
    TimedOut,
    HarnessError,
}

impl TestStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TestStatus::Passed => "pass",
            TestStatus::Failed => "fail",
            TestStatus::CompileError => "build",
            TestStatus::Crashed => "crash",
            TestStatus::TimedOut => "timeout",
            TestStatus::HarnessError => "error",
        }
    }
    pub fn passed(&self) -> bool {
        *self == TestStatus::Passed
    }
}

#[derive(Clone, Debug)]
pub struct TestOutcome {
    pub name: String,
    pub status: TestStatus,
    pub stdin: String,
    pub expected: String,
    pub actual: String,
    pub detail: String,
    pub duration: Duration,
}

/// Compare program output with the expected answer.
///
/// Trailing whitespace on each line and at the end of the output is ignored;
/// interior spacing is not, because `"0 1"` and `"01"` are different answers.
pub fn outputs_match(expected: &str, actual: &str) -> bool {
    let norm = |s: &str| {
        s.replace("\r\n", "\n")
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    };
    norm(expected) == norm(actual)
}

fn classify(run: &RunOutcome, expected: &str) -> TestStatus {
    if run.error.is_some() {
        TestStatus::HarnessError
    } else if run.timed_out {
        TestStatus::TimedOut
    } else if !run.compiled {
        TestStatus::CompileError
    } else if run.exit_code != Some(0) {
        TestStatus::Crashed
    } else if outputs_match(expected, run.output()) {
        TestStatus::Passed
    } else {
        TestStatus::Failed
    }
}

/// Run every test case for one submission. Compilation happens per case, which
/// is wasteful but keeps a crash in one case from poisoning the next; problems
/// ship a handful of cases, so it is not worth the complexity to cache.
pub fn run_tests(
    harness: &Harness,
    lang: &str,
    source: &str,
    fields: &[InputField],
    cases: &[TestCase],
) -> Vec<TestOutcome> {
    cases
        .iter()
        .enumerate()
        .map(|(i, case)| {
            let stdin = serialize_input(fields, &case.input);
            let run = harness.run(lang, source, &stdin);
            let status = classify(&run, &case.expected);
            let detail = match status {
                TestStatus::CompileError => run.compile_output.clone(),
                TestStatus::HarnessError => run.error.clone().unwrap_or_default(),
                TestStatus::Crashed | TestStatus::Failed => run.stderr.clone(),
                _ => String::new(),
            };
            TestOutcome {
                name: if case.name.is_empty() {
                    format!("case {}", i + 1)
                } else {
                    case.name.clone()
                },
                status,
                stdin,
                expected: case.expected.clone(),
                actual: run.output().to_string(),
                detail,
                duration: run.duration,
            }
        })
        .collect()
}

pub fn summary(outcomes: &[TestOutcome]) -> String {
    let passed = outcomes.iter().filter(|o| o.status.passed()).count();
    format!("{passed}/{} passed", outcomes.len())
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn trailing_whitespace_and_line_endings_do_not_fail_a_test() {
        assert!(outputs_match("2 4", "2 4\n"));
        assert!(outputs_match("2 4\n", "2 4"));
        assert!(outputs_match("a\nb", "a\r\nb\r\n"));
        assert!(outputs_match("x  ", "x"));
    }

    #[test]
    fn interior_spacing_still_matters() {
        assert!(!outputs_match("0 1", "01"));
        assert!(!outputs_match("1 2", "2 1"));
        assert!(!outputs_match("a\nb", "b\na"));
    }

    #[test]
    fn statuses_distinguish_the_ways_a_run_can_go_wrong() {
        let base = RunOutcome {
            compiled: true,
            exit_code: Some(0),
            ..Default::default()
        };

        let ok = RunOutcome {
            stdout: "3".into(),
            ..base.clone()
        };
        assert_eq!(classify(&ok, "3"), TestStatus::Passed);
        assert_eq!(classify(&ok, "4"), TestStatus::Failed);

        let build = RunOutcome {
            compiled: false,
            ..base.clone()
        };
        assert_eq!(classify(&build, "3"), TestStatus::CompileError);

        let crash = RunOutcome {
            exit_code: Some(2),
            ..base.clone()
        };
        assert_eq!(classify(&crash, "3"), TestStatus::Crashed);

        let slow = RunOutcome {
            timed_out: true,
            ..base.clone()
        };
        assert_eq!(classify(&slow, "3"), TestStatus::TimedOut);

        assert_eq!(
            classify(&RunOutcome::failed("no toolchain"), "3"),
            TestStatus::HarnessError
        );
    }

    #[test]
    fn a_timeout_outranks_a_missing_binary_in_the_report() {
        // Both flags set: the user should be told it hung, which is the more
        // actionable of the two.
        let both = RunOutcome {
            timed_out: true,
            compiled: false,
            ..Default::default()
        };
        assert_eq!(classify(&both, ""), TestStatus::TimedOut);
    }

    #[test]
    fn summary_counts_passes() {
        let mk = |status| TestOutcome {
            name: "c".into(),
            status,
            stdin: String::new(),
            expected: String::new(),
            actual: String::new(),
            detail: String::new(),
            duration: Duration::ZERO,
        };
        let outcomes = vec![
            mk(TestStatus::Passed),
            mk(TestStatus::Failed),
            mk(TestStatus::Passed),
        ];
        assert_eq!(summary(&outcomes), "2/3 passed");
    }
}
