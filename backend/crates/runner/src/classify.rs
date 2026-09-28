//! Deciding what happened to a run.
//!
//! The watchdogs in `exec` know *why they* stopped a process (time, memory,
//! output). Everything else has to be read from how the process ended, and
//! running out of memory in particular looks different in every language:
//!
//! | language | how an allocation failure surfaces |
//! | --- | --- |
//! | C++ | `std::bad_alloc` → `terminate` → SIGABRT (under `RLIMIT_AS`) |
//! | Python | `MemoryError` traceback, exit 1 (under `RLIMIT_AS`) |
//! | Java | `java.lang.OutOfMemoryError`, exit 1 (heap capped by `-Xmx`) |
//! | Go | `fatal error: runtime: out of memory`, exit 2 — or our RSS watchdog |
//!
//! A message on stderr is only trusted as evidence when the process actually
//! failed: a program that prints "MemoryError" and exits 0 is `Ok`.

use dsa_protocol::CaseStatus;

/// Why a process stopped, as far as the executor knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// It exited or died on its own; see the exit code and signal.
    Exited,
    /// The wall-clock watchdog killed it.
    TimedOut,
    /// A stream went past `max_output_bytes`, so the group was killed.
    OutputExceeded,
    /// The RSS watchdog saw the process group above its memory limit.
    MemoryExceeded,
}

const SIGKILL: i32 = 9;
const SIGXCPU: i32 = 24;

/// Substrings that mean "this process ran out of memory", per runtime.
const OOM_MARKERS: &[&str] = &[
    "std::bad_alloc",
    "java.lang.OutOfMemoryError",
    "MemoryError",
    "runtime: out of memory",
    "fatal error: out of memory",
    "cannot allocate memory",
    "Cannot allocate memory",
    "out of memory allocating heap",
];

/// Whether stderr carries a runtime's out-of-memory report.
pub fn mentions_oom(stderr: &str) -> bool {
    OOM_MARKERS.iter().any(|m| stderr.contains(m))
}

/// Map a finished process to the status the API reports.
///
/// Watchdog verdicts win over everything else, because they are facts rather
/// than inferences. Then signals: `SIGXCPU` is the `RLIMIT_CPU` backstop firing,
/// and a `SIGKILL` nobody on our side sent can only have come from the kernel
/// OOM killer (or the hard CPU limit, which the soft `SIGXCPU` precedes by a
/// second and our wall clock precedes by more). Anything else non-zero is a
/// runtime error unless stderr says it was memory.
pub fn classify(
    termination: Termination,
    exit_code: Option<i32>,
    signal: Option<i32>,
    stderr: &str,
) -> CaseStatus {
    match termination {
        Termination::TimedOut => return CaseStatus::Timeout,
        Termination::OutputExceeded => return CaseStatus::OutputLimit,
        Termination::MemoryExceeded => return CaseStatus::MemoryLimit,
        Termination::Exited => {}
    }
    match (exit_code, signal) {
        (Some(0), _) => CaseStatus::Ok,
        (_, Some(SIGXCPU)) => CaseStatus::Timeout,
        (_, Some(SIGKILL)) => CaseStatus::MemoryLimit,
        _ if mentions_oom(stderr) => CaseStatus::MemoryLimit,
        _ => CaseStatus::RuntimeError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CaseStatus::*;
    use Termination::*;

    #[test]
    fn watchdog_verdicts_are_final() {
        assert_eq!(classify(TimedOut, None, Some(9), ""), Timeout);
        assert_eq!(classify(OutputExceeded, Some(0), None, ""), OutputLimit);
        assert_eq!(classify(MemoryExceeded, None, Some(9), ""), MemoryLimit);
        // Even a clean exit loses to a watchdog that fired first.
        assert_eq!(classify(TimedOut, Some(0), None, ""), Timeout);
    }

    #[test]
    fn a_clean_exit_is_ok_whatever_it_printed() {
        assert_eq!(classify(Exited, Some(0), None, ""), Ok);
        assert_eq!(classify(Exited, Some(0), None, "MemoryError"), Ok);
    }

    #[test]
    fn each_runtime_s_out_of_memory_report_is_recognised() {
        let cases = [
            (None, Some(6), "terminate called after throwing an instance of 'std::bad_alloc'\n  what():  std::bad_alloc"),
            (Some(1), None, "Traceback (most recent call last):\n  File \"main.py\", line 1\nMemoryError"),
            (Some(1), None, "Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space"),
            (Some(2), None, "fatal error: runtime: out of memory\n\ngoroutine 1 [running]:"),
        ];
        for (code, sig, err) in cases {
            assert_eq!(classify(Exited, code, sig, err), MemoryLimit, "{err}");
        }
    }

    #[test]
    fn signals_the_limits_deliver_are_mapped() {
        assert_eq!(classify(Exited, None, Some(SIGXCPU), ""), Timeout);
        assert_eq!(classify(Exited, None, Some(SIGKILL), ""), MemoryLimit);
    }

    #[test]
    fn ordinary_failures_are_runtime_errors() {
        assert_eq!(
            classify(Exited, Some(1), None, "panic: index out of range"),
            RuntimeError
        );
        assert_eq!(classify(Exited, None, Some(11), ""), RuntimeError); // SIGSEGV
        assert_eq!(
            classify(Exited, None, Some(6), "assertion failed"),
            RuntimeError
        ); // SIGABRT
        assert_eq!(classify(Exited, Some(3), None, ""), RuntimeError);
        // A process that could not even be waited for still has to map somewhere.
        assert_eq!(classify(Exited, None, None, ""), RuntimeError);
    }
}
