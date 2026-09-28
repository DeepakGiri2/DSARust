//! Run and test a submission: the Practice tab's ▶ Run and ✓ run tests.
//!
//! The program is assembled here exactly as the desktop assembles it
//! (`content::program`), the runner fleet compiles it once and runs every
//! case, and the *comparison* happens here, with the desktop harness's own
//! rules (`dsa_harness::outputs_match`) — the expected answers never leave the
//! API, so code under test cannot read them.
//!
//! One Run is one attempt; a test sweep where every case passes solves the
//! problem. Submission, attempt, solve and the day's activity are written in a
//! single transaction, so the history and the progress can never disagree.

use crate::content;
use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, Json, Path};
use crate::routes::common::{known_slug, limit, own_profile, premium_entitled, MINUTE};
use crate::runner::RunnerFailure;
use crate::state::AppState;
use crate::store::practice;
use axum::extract::State;
use dsa_protocol::{
    CaseInput, CaseStatus, ExecuteRequest, ExecuteResponse, Limits, PROTOCOL_VERSION,
};
use uuid::Uuid;

/// Output kept per test case in the stored result (the runner already caps
/// what it captures; the history does not need all of it).
const KEEP: usize = 4_000;

/// A run is answered synchronously through CloudFront, whose origin read
/// timeout is 60 s, so compile + every case must fit well inside that:
/// 15 s to build, 5 s per case, 30 s for all cases together.
const RUN_LIMITS: Limits = Limits {
    compile_timeout_ms: 15_000,
    run_timeout_ms: 5_000,
    total_timeout_ms: 30_000,
    memory_mb: 256,
    max_output_bytes: 64 * 1024,
};

struct Case {
    name: String,
    stdin: String,
    expected: Option<String>,
    edge: bool,
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Explain a limit the program hit, in the words the tests table shows.
fn limit_note(status: CaseStatus) -> Option<&'static str> {
    match status {
        CaseStatus::MemoryLimit => Some("memory limit exceeded"),
        CaseStatus::OutputLimit => Some("output limit exceeded — is something printing in a loop?"),
        CaseStatus::Skipped => Some("not run: the time budget for this submission ran out"),
        _ => None,
    }
}

/// Fold the runner's response into what the client shows. Pure, so the
/// classification is tested without a runner.
fn to_output(kind: &str, cases: &[Case], resp: &ExecuteResponse) -> dto::RunOutput {
    let compile = resp.compile.as_ref().map(|c| dto::CompileInfo {
        ok: c.ok,
        output: c.output.clone(),
        duration_ms: c.duration_ms,
    });
    let duration_ms = resp.duration_ms;

    if let Some(err) = &resp.error {
        let msg = err.message.clone();
        return if kind == "run" {
            dto::RunOutput {
                status: "error".into(),
                compile,
                stdout: Some(String::new()),
                stderr: Some(msg),
                exit_code: Some(None),
                timed_out: Some(false),
                tests: None,
                passed: 0,
                total: 1,
                duration_ms,
            }
        } else {
            let tests = cases
                .iter()
                .map(|c| dto::TestResult {
                    name: c.name.clone(),
                    status: "error".into(),
                    stdin: c.stdin.clone(),
                    expected: c.expected.clone().unwrap_or_default(),
                    actual: String::new(),
                    detail: msg.clone(),
                    duration_ms: 0,
                    edge: c.edge,
                })
                .collect::<Vec<_>>();
            dto::RunOutput {
                status: "error".into(),
                compile,
                stdout: None,
                stderr: None,
                exit_code: None,
                timed_out: None,
                passed: 0,
                total: tests.len() as i32,
                tests: Some(tests),
                duration_ms,
            }
        };
    }

    let built = resp.compile.as_ref().is_none_or(|c| c.ok);
    let compile_text = resp
        .compile
        .as_ref()
        .map(|c| c.output.clone())
        .unwrap_or_default();

    if kind == "run" {
        let out = resp.cases.first();
        let status = if !built {
            "compile_error"
        } else {
            match out.map(|o| o.status) {
                Some(CaseStatus::Ok) => "ok",
                Some(CaseStatus::Timeout) | Some(CaseStatus::Skipped) => "timeout",
                Some(_) => "runtime_error",
                None => "error",
            }
        };
        let mut stderr = out.map(|o| o.stderr.clone()).unwrap_or_default();
        if let Some(note) = out.and_then(|o| limit_note(o.status)) {
            if !stderr.is_empty() && !stderr.ends_with('\n') {
                stderr.push('\n');
            }
            stderr.push_str(note);
        }
        return dto::RunOutput {
            status: status.into(),
            compile,
            stdout: Some(out.map(|o| o.stdout.clone()).unwrap_or_default()),
            stderr: Some(stderr),
            exit_code: Some(out.and_then(|o| o.exit_code)),
            timed_out: Some(out.is_some_and(|o| matches!(o.status, CaseStatus::Timeout))),
            tests: None,
            passed: i32::from(status == "ok"),
            total: 1,
            duration_ms,
        };
    }

    // kind = test: the desktop's `classify`, case by case.
    let tests: Vec<dto::TestResult> = cases
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let expected = c.expected.clone().unwrap_or_default();
            let o = resp.cases.iter().find(|o| o.id == format!("c{i}"));
            let (status, actual, mut detail, ms) = match (built, o) {
                (false, _) => ("build", String::new(), compile_text.clone(), 0),
                (true, None) => (
                    "error",
                    String::new(),
                    "the runner returned no result for this case".into(),
                    0,
                ),
                (true, Some(o)) => {
                    let actual = o.stdout.trim_end().to_string();
                    let status = match o.status {
                        CaseStatus::Timeout | CaseStatus::Skipped => "timeout",
                        CaseStatus::Ok if dsa_harness::outputs_match(&expected, &actual) => "pass",
                        CaseStatus::Ok => "fail",
                        _ => "crash",
                    };
                    (status, actual, o.stderr.clone(), o.duration_ms)
                }
            };
            if let Some(note) = o.and_then(|o| limit_note(o.status)) {
                if !detail.is_empty() {
                    detail.push('\n');
                }
                detail.push_str(note);
            }
            dto::TestResult {
                name: c.name.clone(),
                status: status.into(),
                stdin: c.stdin.clone(),
                expected,
                actual: clip(&actual, KEEP),
                detail: clip(&detail, KEEP),
                duration_ms: ms,
                edge: c.edge,
            }
        })
        .collect();
    let passed = tests.iter().filter(|t| t.status == "pass").count() as i32;
    let total = tests.len() as i32;
    let status = if !built {
        "compile_error"
    } else if passed == total {
        "passed"
    } else {
        "failed"
    };
    dto::RunOutput {
        status: status.into(),
        compile,
        stdout: None,
        stderr: None,
        exit_code: None,
        timed_out: None,
        tests: Some(tests),
        passed,
        total,
        duration_ms,
    }
}

pub async fn run(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Json(req): Json<dto::RunRequest>,
) -> ApiResult<axum::Json<dto::RunResult>> {
    own_profile(&state, &authed, pid).await?;
    known_slug(&state, &req.slug)?;
    let pack = state
        .content
        .pack(&req.slug)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    if !matches!(req.kind.as_str(), "run" | "test") {
        return Err(ApiError::field("kind", "kind is run or test"));
    }
    if !matches!(req.mode.as_str(), "solution" | "program") {
        return Err(ApiError::field("mode", "mode is solution or program"));
    }
    let lang = state
        .content
        .language(&req.lang)
        .ok_or_else(|| ApiError::not_found("language"))?;
    if !state.runner.enabled() {
        return Err(ApiError::Unavailable(
            "Running code isn't enabled on this server.".into(),
        ));
    }
    if !state.runner.languages().contains(&lang.id) {
        return Err(ApiError::Unavailable(format!(
            "{} can't be run on this server yet.",
            lang.label
        )));
    }
    if req.code.len() > dsa_protocol::MAX_SOURCE_BYTES {
        return Err(ApiError::field("code", "That program is too large to run."));
    }
    if req
        .stdin
        .as_ref()
        .is_some_and(|s| s.len() > dsa_protocol::MAX_STDIN_BYTES)
    {
        return Err(ApiError::field("stdin", "That input is too large."));
    }
    let entitled = premium_entitled(&authed.user);
    if state.content.is_premium(pack.meta.tier) && !entitled {
        return Err(ApiError::PaymentRequired(
            "This problem is part of Pro.".into(),
        ));
    }
    if state.cfg.policy.require_verified_email && !authed.user.verified() {
        return Err(ApiError::EmailUnverified);
    }
    let per_min = if entitled {
        state.cfg.policy.runs_pro_per_min
    } else {
        state.cfg.policy.runs_free_per_min
    };
    limit(&state, &format!("runs:{}", authed.user.id), per_min, MINUTE).await?;

    let fields = &pack.meta.inputs;
    let cases: Vec<Case> = if req.kind == "run" {
        vec![Case {
            name: "run".into(),
            stdin: req
                .stdin
                .clone()
                .unwrap_or_else(|| dsa_harness::serialize_input(fields, &pack.meta.default_input)),
            expected: None,
            edge: false,
        }]
    } else {
        pack.meta
            .tests
            .iter()
            .enumerate()
            .map(|(i, t)| Case {
                name: if t.name.is_empty() {
                    format!("case {}", i + 1)
                } else {
                    t.name.clone()
                },
                stdin: dsa_harness::serialize_input(fields, &t.input),
                expected: Some(t.expected.clone()),
                edge: t.edge,
            })
            .collect()
    };
    if cases.is_empty() {
        return Err(ApiError::BadRequest(
            "This problem has no test cases yet.".into(),
        ));
    }

    let id = Uuid::now_v7();
    let source = content::program(pack, &lang.id, &req.code, req.mode == "program");
    let exec = ExecuteRequest {
        protocol: PROTOCOL_VERSION,
        job_id: id.to_string(),
        language: lang.id.clone(),
        source,
        cases: cases
            .iter()
            .enumerate()
            .map(|(i, c)| CaseInput {
                id: format!("c{i}"),
                stdin: c.stdin.clone(),
            })
            .collect(),
        limits: RUN_LIMITS,
    };
    exec.validate().map_err(ApiError::BadRequest)?;
    let resp = state.runner.execute(exec).await.map_err(|e| match e {
        RunnerFailure::Busy => {
            ApiError::Unavailable("Every code runner is busy — try again in a few seconds.".into())
        }
        RunnerFailure::Unavailable(m) => {
            tracing::error!(error = %m, "runner unavailable");
            ApiError::Unavailable("Code execution is temporarily unavailable.".into())
        }
    })?;
    let output = to_output(&req.kind, &cases, &resp);
    metrics::counter!("dsa_runs_total", "kind" => req.kind.clone(), "lang" => lang.id.clone(), "status" => output.status.clone())
        .increment(1);

    let solved = req.kind == "test" && output.status == "passed";
    let mut tx = state.db.begin().await?;
    let created_at = practice::insert_submission(
        &mut tx,
        practice::NewSubmission {
            id,
            profile: pid,
            user: authed.user.id,
            slug: &req.slug,
            lang: &lang.id,
            kind: &req.kind,
            mode: &req.mode,
            code: &req.code,
            status: &output.status,
            passed: output.passed,
            total: output.total,
            duration_ms: output.duration_ms.min(i32::MAX as u64) as i32,
            result: serde_json::to_value(&output).map_err(ApiError::internal)?,
        },
    )
    .await?;
    let (progress, newly) = practice::record_attempt(&mut tx, pid, &req.slug, solved).await?;
    practice::bump_activity(&mut tx, pid, &authed.user.timezone, 1, i32::from(newly)).await?;
    tx.commit().await?;

    Ok(axum::Json(dto::RunResult {
        id,
        slug: req.slug,
        lang: lang.id.clone(),
        kind: req.kind,
        mode: req.mode,
        output,
        progress,
        created_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_protocol::{CaseOutcome, CompileOutcome, RunnerError, RunnerErrorCode};

    fn case(expected: &str) -> Case {
        Case {
            name: "c".into(),
            stdin: "1\n".into(),
            expected: Some(expected.into()),
            edge: false,
        }
    }

    fn outcome(id: &str, status: CaseStatus, stdout: &str) -> CaseOutcome {
        CaseOutcome {
            id: id.into(),
            status,
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: Some(if status == CaseStatus::Ok { 0 } else { 1 }),
            signal: None,
            duration_ms: 3,
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    fn resp(compile_ok: bool, cases: Vec<CaseOutcome>) -> ExecuteResponse {
        ExecuteResponse {
            protocol: PROTOCOL_VERSION,
            job_id: "j".into(),
            language: "go".into(),
            compile: Some(CompileOutcome {
                ok: compile_ok,
                output: if compile_ok {
                    String::new()
                } else {
                    "main.go:3: undefined: x".into()
                },
                timed_out: false,
                truncated: false,
                duration_ms: 10,
            }),
            cases,
            error: None,
            runner_version: "t".into(),
            duration_ms: 20,
        }
    }

    #[test]
    fn a_full_pass_is_passed_and_whitespace_is_forgiven() {
        let cases = vec![case("0 1"), case("2 3")];
        let r = resp(
            true,
            vec![
                outcome("c0", CaseStatus::Ok, "0 1\n"),
                outcome("c1", CaseStatus::Ok, "2 3  \n"),
            ],
        );
        let out = to_output("test", &cases, &r);
        assert_eq!(out.status, "passed");
        assert_eq!((out.passed, out.total), (2, 2));
    }

    #[test]
    fn each_failure_mode_gets_its_own_status() {
        let cases = vec![case("1"), case("2"), case("3")];
        let r = resp(
            true,
            vec![
                outcome("c0", CaseStatus::Ok, "9"),
                outcome("c1", CaseStatus::Timeout, ""),
                outcome("c2", CaseStatus::RuntimeError, ""),
            ],
        );
        let out = to_output("test", &cases, &r);
        let s: Vec<_> = out
            .tests
            .as_ref()
            .unwrap()
            .iter()
            .map(|t| t.status.as_str())
            .collect();
        assert_eq!(s, ["fail", "timeout", "crash"]);
        assert_eq!(out.status, "failed");
    }

    #[test]
    fn a_build_failure_marks_every_case_build_with_the_diagnostics() {
        let cases = vec![case("1"), case("2")];
        let out = to_output("test", &cases, &resp(false, vec![]));
        assert_eq!(out.status, "compile_error");
        let t = out.tests.unwrap();
        assert!(t
            .iter()
            .all(|t| t.status == "build" && t.detail.contains("undefined: x")));
    }

    #[test]
    fn a_plain_run_reports_stdout_and_limits() {
        let cases = vec![Case {
            name: "run".into(),
            stdin: String::new(),
            expected: None,
            edge: false,
        }];
        let out = to_output(
            "run",
            &cases,
            &resp(true, vec![outcome("c0", CaseStatus::MemoryLimit, "")]),
        );
        assert_eq!(out.status, "runtime_error");
        assert!(out.stderr.unwrap().contains("memory limit"));
        let ok = to_output(
            "run",
            &cases,
            &resp(true, vec![outcome("c0", CaseStatus::Ok, "hi\n")]),
        );
        assert_eq!(ok.status, "ok");
        assert_eq!(ok.stdout.as_deref(), Some("hi\n"));
    }

    #[test]
    fn a_runner_refusal_is_an_error_not_a_wrong_answer() {
        let mut r = resp(true, vec![]);
        r.error = Some(RunnerError::new(
            RunnerErrorCode::Internal,
            "sandbox unavailable",
        ));
        let out = to_output("test", &[case("1")], &r);
        assert_eq!(out.status, "error");
        assert_eq!(out.tests.unwrap()[0].status, "error");
    }
}
