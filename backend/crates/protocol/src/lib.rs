//! The wire contract between `dsa-api` and `dsa-runner`.
//!
//! The API is trusted: it owns the database, the problem content and the
//! expected answers. The runner is not: it executes whatever a user typed, so
//! it is given exactly what it needs to do that and nothing more.
//!
//! * The request names a **language id**, never a command line. The runner has
//!   its own toolchain registry baked into its image, so even a compromised API
//!   cannot make it execute an arbitrary argv outside the sandbox.
//! * The request carries **stdin only**. Expected outputs never leave the API,
//!   which compares them itself (`dsa_harness::outputs_match`) — so the answers
//!   to hidden tests cannot be read by the code being judged.
//! * Limits in the request are **requests**, not orders: the runner clamps
//!   every one of them to its own configured ceiling.
//!
//! The same JSON travels over HTTP (`POST /v1/execute`, local and ECS) and as
//! a Lambda invocation payload (AWS), so the transport is not part of the
//! contract.

use serde::{Deserialize, Serialize};

/// Bumped on any incompatible change. The runner rejects a request whose
/// version it does not speak rather than guessing.
pub const PROTOCOL_VERSION: u32 = 1;

/// Upper bound on test cases in one job, enforced on both sides.
pub const MAX_CASES: usize = 64;

/// Upper bound on the program text, enforced on both sides (bytes).
pub const MAX_SOURCE_BYTES: usize = 128 * 1024;

/// Upper bound on one case's stdin (bytes).
pub const MAX_STDIN_BYTES: usize = 256 * 1024;

// ─────────────────────────────────────────────────────────────────────────────
// Request
// ─────────────────────────────────────────────────────────────────────────────

/// Compile `source` once, then run it once per case with that case's stdin.
///
/// Compiling once and running each case as a fresh process in a fresh working
/// directory is both faster than the desktop's compile-per-case and just as
/// isolated: nothing one case leaves behind is visible to the next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub protocol: u32,
    /// Opaque id for log correlation (the API's run id). Echoed back.
    pub job_id: String,
    /// A language id from the runner's registry: `go`, `cpp`, `java`, `python`.
    pub language: String,
    /// The complete program — harness and user code already assembled.
    pub source: String,
    pub cases: Vec<CaseInput>,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseInput {
    /// Echoed back so results can be matched to cases without relying on order.
    pub id: String,
    pub stdin: String,
}

/// Resource limits the caller asks for. Every field is clamped by the runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    /// Wall-clock budget for the compile step.
    pub compile_timeout_ms: u64,
    /// Wall-clock budget for each case.
    pub run_timeout_ms: u64,
    /// Budget for the whole job; cases that cannot start inside it are skipped.
    pub total_timeout_ms: u64,
    /// Memory per process, in MiB.
    pub memory_mb: u64,
    /// Captured bytes per stream (stdout, stderr) per case.
    pub max_output_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            compile_timeout_ms: 20_000,
            run_timeout_ms: 5_000,
            total_timeout_ms: 45_000,
            memory_mb: 256,
            max_output_bytes: 64 * 1024,
        }
    }
}

impl ExecuteRequest {
    /// Structural checks both sides agree on. The runner applies these before
    /// touching a toolchain; the API applies them before paying for a call.
    pub fn validate(&self) -> Result<(), String> {
        if self.protocol != PROTOCOL_VERSION {
            return Err(format!(
                "protocol {} is not supported (expected {PROTOCOL_VERSION})",
                self.protocol
            ));
        }
        if self.language.is_empty()
            || !self
                .language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("language must be a registry id".into());
        }
        if self.source.len() > MAX_SOURCE_BYTES {
            return Err(format!("source exceeds {MAX_SOURCE_BYTES} bytes"));
        }
        if self.cases.is_empty() {
            return Err("at least one case is required".into());
        }
        if self.cases.len() > MAX_CASES {
            return Err(format!("at most {MAX_CASES} cases per job"));
        }
        if let Some(c) = self.cases.iter().find(|c| c.stdin.len() > MAX_STDIN_BYTES) {
            return Err(format!(
                "stdin of case \"{}\" exceeds {MAX_STDIN_BYTES} bytes",
                c.id
            ));
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Response
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub protocol: u32,
    pub job_id: String,
    pub language: String,
    /// `None` for languages with no compile step (Python).
    pub compile: Option<CompileOutcome>,
    /// One per request case, in request order. Empty when compilation failed
    /// or the runner refused the job.
    pub cases: Vec<CaseOutcome>,
    /// Set when the runner itself could not do the job. A user's program
    /// failing is *not* an error here — that is a case status.
    pub error: Option<RunnerError>,
    pub runner_version: String,
    /// Wall time the runner spent on the whole job.
    pub duration_ms: u64,
}

impl ExecuteResponse {
    /// A refusal: no compile, no cases, just the reason.
    pub fn refused(req_job: &str, language: &str, error: RunnerError) -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            job_id: req_job.to_string(),
            language: language.to_string(),
            compile: None,
            cases: Vec::new(),
            error: Some(error),
            runner_version: String::new(),
            duration_ms: 0,
        }
    }

    /// True when the program built (or needed no build).
    pub fn compiled(&self) -> bool {
        self.error.is_none() && self.compile.as_ref().is_none_or(|c| c.ok)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileOutcome {
    pub ok: bool,
    /// Compiler diagnostics (stdout and stderr interleaved), truncated.
    pub output: String,
    pub timed_out: bool,
    pub truncated: bool,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    /// Exited with status 0. Whether the output is *right* is the API's call.
    Ok,
    /// Non-zero exit or killed by a signal the program brought on itself.
    RuntimeError,
    /// Exceeded the wall-clock or CPU budget.
    Timeout,
    /// Exceeded the memory limit.
    MemoryLimit,
    /// Wrote more than `max_output_bytes` to a stream.
    OutputLimit,
    /// Not run: the job's total budget ran out first.
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseOutcome {
    pub id: String,
    pub status: CaseStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub duration_ms: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerErrorCode {
    /// The request failed [`ExecuteRequest::validate`].
    BadRequest,
    /// The language id is not in this runner's registry.
    UnsupportedLanguage,
    /// Every execution slot is taken. Retry elsewhere or later.
    Busy,
    /// The runner failed for a reason that is not the program's fault.
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerError {
    pub code: RunnerErrorCode,
    pub message: String,
}

impl RunnerError {
    pub fn new(code: RunnerErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Health
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /healthz` on an HTTP runner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunnerHealth {
    pub status: String,
    pub version: String,
    pub protocol: u32,
    /// Languages this runner can execute, with the toolchain version it found.
    pub languages: Vec<LanguageInfo>,
    /// Maximum concurrent jobs.
    pub capacity: usize,
    pub in_flight: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub id: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> ExecuteRequest {
        ExecuteRequest {
            protocol: PROTOCOL_VERSION,
            job_id: "job-1".into(),
            language: "go".into(),
            source: "package main\nfunc main() {}\n".into(),
            cases: vec![CaseInput {
                id: "c1".into(),
                stdin: "2 7\n9\n".into(),
            }],
            limits: Limits::default(),
        }
    }

    #[test]
    fn a_well_formed_request_validates() {
        assert!(req().validate().is_ok());
    }

    #[test]
    fn structural_limits_are_enforced() {
        let mut r = req();
        r.protocol = 99;
        assert!(r.validate().unwrap_err().contains("protocol"));

        let mut r = req();
        r.cases.clear();
        assert!(r.validate().is_err());

        let mut r = req();
        r.language = "go; rm -rf /".into();
        assert!(r.validate().is_err(), "a language is an id, not a command");

        let mut r = req();
        r.source = "x".repeat(MAX_SOURCE_BYTES + 1);
        assert!(r.validate().is_err());

        let mut r = req();
        r.cases = (0..=MAX_CASES)
            .map(|i| CaseInput {
                id: i.to_string(),
                stdin: String::new(),
            })
            .collect();
        assert!(r.validate().is_err());
    }

    #[test]
    fn limits_default_when_omitted() {
        let j = r#"{"protocol":1,"job_id":"j","language":"cpp","source":"","cases":[{"id":"a","stdin":""}]}"#;
        let r: ExecuteRequest = serde_json::from_str(j).unwrap();
        assert_eq!(r.limits, Limits::default());
        let partial = r#"{"protocol":1,"job_id":"j","language":"cpp","source":"","cases":[],"limits":{"memory_mb":64}}"#;
        let r: ExecuteRequest = serde_json::from_str(partial).unwrap();
        assert_eq!(r.limits.memory_mb, 64);
        assert_eq!(r.limits.run_timeout_ms, Limits::default().run_timeout_ms);
    }

    #[test]
    fn statuses_use_snake_case_on_the_wire() {
        let j = serde_json::to_string(&CaseStatus::MemoryLimit).unwrap();
        assert_eq!(j, "\"memory_limit\"");
        let j = serde_json::to_string(&RunnerErrorCode::UnsupportedLanguage).unwrap();
        assert_eq!(j, "\"unsupported_language\"");
    }

    #[test]
    fn a_refusal_did_not_compile() {
        let r = ExecuteResponse::refused(
            "j",
            "go",
            RunnerError::new(RunnerErrorCode::Busy, "all slots taken"),
        );
        assert!(!r.compiled());
        assert!(r.cases.is_empty());
    }
}
