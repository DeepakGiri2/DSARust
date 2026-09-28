//! `lambda` mode: the transport for AWS, selected when `AWS_LAMBDA_RUNTIME_API`
//! is set.
//!
//! The invocation payload is an `ExecuteRequest` and the result an
//! `ExecuteResponse`, byte for byte what `http` mode carries. Lambda sends one
//! invocation at a time to an execution environment, so there is exactly one
//! job per invocation and the slot pool (capacity 1 here: the runner is not
//! root) never reports busy in practice.
//!
//! Nothing here needs a network: the runtime API is served from inside the
//! execution environment, and the function sits in isolated subnets with no
//! egress at all.

use crate::engine::Engine;
use dsa_protocol::{ExecuteRequest, ExecuteResponse, RunnerError, RunnerErrorCode};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::sync::Arc;

pub async fn serve(engine: Arc<Engine>) -> anyhow::Result<()> {
    tracing::info!("lambda runner ready");
    lambda_runtime::run(service_fn(move |event: LambdaEvent<serde_json::Value>| {
        let engine = Arc::clone(&engine);
        async move { Ok::<_, Error>(handle(&engine, event.payload).await) }
    }))
    .await
    .map_err(|e| anyhow::anyhow!("lambda runtime: {e}"))
}

/// The payload is taken as raw JSON and parsed here, not by the runtime, so a
/// malformed request gets a `bad_request` response in the protocol's shape
/// instead of an opaque function error.
async fn handle(engine: &Engine, payload: serde_json::Value) -> ExecuteResponse {
    let req: ExecuteRequest = match serde_json::from_value(payload) {
        Ok(req) => req,
        Err(e) => return refused(RunnerErrorCode::BadRequest, format!("invalid request: {e}")),
    };
    match engine.try_acquire() {
        Some(slot) => engine.execute(req, &slot).await,
        None => refused(
            RunnerErrorCode::Busy,
            "the runner is already executing a job",
        ),
    }
}

fn refused(code: RunnerErrorCode, message: impl Into<String>) -> ExecuteResponse {
    let mut resp = ExecuteResponse::refused("", "", RunnerError::new(code, message));
    resp.runner_version = crate::RUNNER_VERSION.to_string();
    resp
}
