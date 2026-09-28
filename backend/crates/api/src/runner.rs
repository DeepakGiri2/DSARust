//! The API's side of code execution: turning a Run into an `ExecuteRequest`
//! for the sandboxed runner fleet and getting an `ExecuteResponse` back.
//!
//! Transports:
//!
//! * **http** — a `dsa-runner` in http mode (docker-compose locally, or ECS).
//! * **lambda** — the runner as an AWS Lambda container function, invoked with
//!   the task role. Each invocation is its own microVM with no network route,
//!   and Lambda scales it to the burst without a queue of our own.
//! * **local** — development only: the desktop's harness (`dsa-harness`) on
//!   this machine's toolchains, with no sandbox at all. Refused in production
//!   by `Config::from_env`.
//!
//! In front of every transport sits a semaphore: an API task never has more
//! than `RUNNER_CONCURRENCY` executions in flight, and a request that cannot
//! get a slot within a couple of seconds is told the fleet is busy instead of
//! queueing without bound.

use crate::config::{Config, RunnerMode};
use async_trait::async_trait;
use dsa_protocol::{
    CaseOutcome, CaseStatus, CompileOutcome, ExecuteRequest, ExecuteResponse, RunnerErrorCode,
    PROTOCOL_VERSION,
};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Debug, thiserror::Error)]
pub enum RunnerFailure {
    #[error("the code runners are busy")]
    Busy,
    #[error("code execution is unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
trait Transport: Send + Sync {
    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResponse, RunnerFailure>;
}

pub struct Runner {
    transport: Option<Box<dyn Transport>>,
    permits: Arc<Semaphore>,
    languages: BTreeSet<String>,
    pub label: &'static str,
}

impl Runner {
    pub async fn from_config(
        cfg: &Config,
        http: reqwest::Client,
        known: &[dsa_core::problem::LanguageDef],
    ) -> anyhow::Result<Self> {
        let all: BTreeSet<String> = known.iter().map(|l| l.id.clone()).collect();
        let (transport, languages, label): (Option<Box<dyn Transport>>, BTreeSet<String>, _) =
            match &cfg.runner {
                RunnerMode::Disabled => (None, BTreeSet::new(), "disabled"),
                RunnerMode::Http { url, token } => (
                    Some(Box::new(HttpTransport {
                        url: url.join("v1/execute")?,
                        token: token.clone(),
                        http,
                    })),
                    all,
                    "http",
                ),
                RunnerMode::Lambda { function } => {
                    (Some(lambda_transport(cfg, function).await?), all, "lambda")
                }
                RunnerMode::Local => {
                    let t = LocalTransport::new(known);
                    let langs = t.languages();
                    (Some(Box::new(t)), langs, "local")
                }
            };
        Ok(Self {
            transport,
            permits: Arc::new(Semaphore::new(cfg.runner_concurrency.max(1))),
            languages,
            label,
        })
    }

    /// A runner that refuses everything — for tests and disabled deployments.
    pub fn disabled() -> Self {
        Self {
            transport: None,
            permits: Arc::new(Semaphore::new(1)),
            languages: BTreeSet::new(),
            label: "disabled",
        }
    }

    pub fn enabled(&self) -> bool {
        self.transport.is_some()
    }

    /// Language ids this deployment can execute.
    pub fn languages(&self) -> &BTreeSet<String> {
        &self.languages
    }

    pub async fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, RunnerFailure> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| RunnerFailure::Unavailable("not configured on this server".into()))?;
        let _slot =
            tokio::time::timeout(Duration::from_secs(2), self.permits.clone().acquire_owned())
                .await
                .map_err(|_| RunnerFailure::Busy)?
                .map_err(|_| RunnerFailure::Unavailable("shutting down".into()))?;

        let started = std::time::Instant::now();
        let mut res = transport.execute(&req).await;
        // A runner reporting Busy (http mode: every slot on that task taken)
        // is worth one retry — the load balancer will likely pick another.
        if matches!(&res, Ok(r) if r.error.as_ref().is_some_and(|e| e.code == RunnerErrorCode::Busy))
        {
            tokio::time::sleep(Duration::from_millis(150 + rand::random::<u64>() % 250)).await;
            res = transport.execute(&req).await;
        }
        metrics::histogram!("dsa_runner_seconds", "transport" => self.label)
            .record(started.elapsed().as_secs_f64());
        match res {
            Ok(r)
                if r.error
                    .as_ref()
                    .is_some_and(|e| e.code == RunnerErrorCode::Busy) =>
            {
                Err(RunnerFailure::Busy)
            }
            other => other,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// http
// ─────────────────────────────────────────────────────────────────────────────

struct HttpTransport {
    url: url::Url,
    token: Option<String>,
    http: reqwest::Client,
}

#[async_trait]
impl Transport for HttpTransport {
    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResponse, RunnerFailure> {
        let budget =
            Duration::from_millis(req.limits.total_timeout_ms + req.limits.compile_timeout_ms)
                + Duration::from_secs(10);
        let mut call = self.http.post(self.url.clone()).json(req).timeout(budget);
        if let Some(t) = &self.token {
            call = call.bearer_auth(t);
        }
        let resp = call
            .send()
            .await
            .map_err(|e| RunnerFailure::Unavailable(format!("runner unreachable: {e}")))?;
        let status = resp.status();
        // 503 carries a Busy refusal in the protocol's own shape.
        let body: Result<ExecuteResponse, _> = resp.json().await;
        match body {
            Ok(r) => Ok(r),
            Err(e) if status.is_success() => Err(RunnerFailure::Unavailable(format!(
                "bad runner response: {e}"
            ))),
            Err(_) => Err(RunnerFailure::Unavailable(format!(
                "runner answered {status}"
            ))),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// lambda
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "aws")]
async fn lambda_transport(cfg: &Config, function: &str) -> anyhow::Result<Box<dyn Transport>> {
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(region) = &cfg.aws_region {
        loader = loader.region(aws_config::Region::new(region.clone()));
    }
    let sdk = loader.load().await;
    Ok(Box::new(LambdaTransport {
        client: aws_sdk_lambda::Client::new(&sdk),
        function: function.to_string(),
    }))
}

#[cfg(not(feature = "aws"))]
async fn lambda_transport(_cfg: &Config, _function: &str) -> anyhow::Result<Box<dyn Transport>> {
    anyhow::bail!("RUNNER_MODE=lambda needs a build with `--features aws`")
}

#[cfg(feature = "aws")]
struct LambdaTransport {
    client: aws_sdk_lambda::Client,
    function: String,
}

#[cfg(feature = "aws")]
#[async_trait]
impl Transport for LambdaTransport {
    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResponse, RunnerFailure> {
        use aws_sdk_lambda::primitives::Blob;
        let payload =
            serde_json::to_vec(req).map_err(|e| RunnerFailure::Unavailable(e.to_string()))?;
        let out = self
            .client
            .invoke()
            .function_name(&self.function)
            .payload(Blob::new(payload))
            .send()
            .await
            .map_err(|e| match e.as_service_error() {
                Some(se) if se.is_too_many_requests_exception() => RunnerFailure::Busy,
                _ => RunnerFailure::Unavailable(format!("lambda invoke failed: {e}")),
            })?;
        if let Some(err) = out.function_error() {
            return Err(RunnerFailure::Unavailable(format!(
                "runner function error: {err}"
            )));
        }
        let bytes = out
            .payload()
            .map(|b| b.as_ref().to_vec())
            .unwrap_or_default();
        serde_json::from_slice(&bytes)
            .map_err(|e| RunnerFailure::Unavailable(format!("bad runner response: {e}")))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// local (development only)
// ─────────────────────────────────────────────────────────────────────────────

struct LocalTransport {
    harness: Arc<dsa_harness::Harness>,
    languages: Vec<dsa_core::problem::LanguageDef>,
}

impl LocalTransport {
    fn new(known: &[dsa_core::problem::LanguageDef]) -> Self {
        let mut harness = dsa_harness::Harness::detect(known);
        // Sending code to a third party is not something a development
        // server should do behind the developer's back.
        harness.allow_remote = std::env::var("RUNNER_LOCAL_ALLOW_REMOTE").is_ok_and(|v| v == "1");
        tracing::warn!(
            "RUNNER_MODE=local: user code runs UNSANDBOXED on this machine (development only)"
        );
        Self {
            harness: Arc::new(harness),
            languages: known.to_vec(),
        }
    }

    fn languages(&self) -> BTreeSet<String> {
        self.languages
            .iter()
            .filter(|l| self.harness.backend(&l.id) != dsa_harness::Backend::Unavailable)
            .map(|l| l.id.clone())
            .collect()
    }
}

#[async_trait]
impl Transport for LocalTransport {
    async fn execute(&self, req: &ExecuteRequest) -> Result<ExecuteResponse, RunnerFailure> {
        let harness = self.harness.clone();
        let req = req.clone();
        tokio::task::spawn_blocking(move || local_execute(&harness, &req))
            .await
            .map_err(|e| RunnerFailure::Unavailable(e.to_string()))
    }
}

/// The desktop harness compiles per run; fold its per-case outcomes into the
/// protocol's compile-once shape.
fn local_execute(harness: &dsa_harness::Harness, req: &ExecuteRequest) -> ExecuteResponse {
    let started = std::time::Instant::now();
    let timeout = Duration::from_millis(req.limits.run_timeout_ms + req.limits.compile_timeout_ms);
    let cap = req.limits.max_output_bytes as usize;
    let mut compile: Option<CompileOutcome> = None;
    let mut cases = Vec::new();
    for case in &req.cases {
        let out = harness.run_with_timeout(&req.language, &req.source, &case.stdin, timeout);
        if let Some(err) = out.error {
            return ExecuteResponse::refused(
                &req.job_id,
                &req.language,
                dsa_protocol::RunnerError::new(RunnerErrorCode::Internal, err),
            );
        }
        if compile.is_none() {
            compile = Some(CompileOutcome {
                ok: out.compiled,
                output: truncate(&out.compile_output, cap).0,
                timed_out: out.timed_out && !out.compiled,
                truncated: out.compile_output.len() > cap,
                duration_ms: 0,
            });
        }
        if !out.compiled {
            break;
        }
        let (stdout, so_t) = truncate(&out.stdout, cap);
        let (stderr, se_t) = truncate(&out.stderr, cap);
        cases.push(CaseOutcome {
            id: case.id.clone(),
            status: if out.timed_out {
                CaseStatus::Timeout
            } else if out.exit_code == Some(0) {
                CaseStatus::Ok
            } else {
                CaseStatus::RuntimeError
            },
            stdout,
            stderr,
            exit_code: out.exit_code,
            signal: None,
            duration_ms: out.duration.as_millis() as u64,
            stdout_truncated: so_t,
            stderr_truncated: se_t,
        });
    }
    ExecuteResponse {
        protocol: PROTOCOL_VERSION,
        job_id: req.job_id.clone(),
        language: req.language.clone(),
        compile,
        cases,
        error: None,
        runner_version: format!("local-harness/{}", env!("CARGO_PKG_VERSION")),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn truncate(s: &str, max: usize) -> (String, bool) {
    if s.len() <= max {
        return (s.to_string(), false);
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_string(), true)
}
