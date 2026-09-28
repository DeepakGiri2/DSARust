//! `http` mode: the transport for docker-compose and ECS.
//!
//! * `POST /v1/execute` — `ExecuteRequest` → `ExecuteResponse`. Every answer,
//!   error or not, carries an `ExecuteResponse` body, so the API parses one
//!   shape; the status code only summarises it.
//! * `GET /healthz` — `RunnerHealth`. Unauthenticated, so container health
//!   checks work without the token; it reveals versions and load, nothing a
//!   job could use.
//!
//! A full runner answers `503 busy` immediately rather than queueing: the API
//! is the one that knows whether to retry, wait or give up, and a queue here
//! would only turn overload into timeouts.

use crate::engine::Engine;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use dsa_protocol::{ExecuteRequest, ExecuteResponse, RunnerError, RunnerErrorCode};
use std::future::IntoFuture;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// Slack on top of the longest job for reading the request and tearing down.
const REQUEST_SLACK: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct AppState {
    engine: Arc<Engine>,
}

/// Build the router. Public so tests can drive it without a socket.
pub fn router(engine: Arc<Engine>) -> Router {
    let config = engine.config();
    let request_timeout = config.ceilings.total_timeout + REQUEST_SLACK;
    let body_limit = config.max_body_bytes;
    let state = AppState { engine };

    let execute = Router::new()
        .route("/v1/execute", post(execute))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token));

    Router::new()
        .route("/healthz", get(healthz))
        .merge(execute)
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Serve until SIGTERM/SIGINT, then drain in-flight jobs for at most one job
/// length before exiting anyway (the orchestrator would kill us soon after).
pub async fn serve(engine: Arc<Engine>) -> anyhow::Result<()> {
    let bind = engine.config().bind.clone();
    let drain_limit = engine.config().ceilings.total_timeout + REQUEST_SLACK;
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, capacity = engine.health().capacity, "http runner listening");

    let (signalled_tx, signalled_rx) = tokio::sync::oneshot::channel::<()>();
    let server = axum::serve(listener, router(engine))
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            tracing::info!("shutting down: finishing in-flight jobs");
            let _ = signalled_tx.send(());
        })
        .into_future();
    tokio::select! {
        result = server => result?,
        () = async {
            if signalled_rx.await.is_ok() {
                tokio::time::sleep(drain_limit).await;
            } else {
                std::future::pending::<()>().await;
            }
        } => tracing::warn!("in-flight jobs did not finish in time; exiting anyway"),
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = term.recv() => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "cannot listen for SIGTERM; only Ctrl-C will stop the runner");
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn healthz(State(state): State<AppState>) -> Json<dsa_protocol::RunnerHealth> {
    Json(state.engine.health())
}

async fn execute(
    State(state): State<AppState>,
    body: Result<Json<ExecuteRequest>, JsonRejection>,
) -> Response {
    let req = match body {
        Ok(Json(req)) => req,
        Err(rejection) => {
            let status = rejection.status();
            return refusal(
                status,
                "",
                "",
                RunnerErrorCode::BadRequest,
                rejection.body_text(),
            );
        }
    };
    let Some(slot) = state.engine.try_acquire() else {
        let mut resp = refusal(
            StatusCode::SERVICE_UNAVAILABLE,
            &req.job_id,
            &req.language,
            RunnerErrorCode::Busy,
            "every execution slot is busy",
        );
        resp.headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
        return resp;
    };
    // If the client goes away, this future is dropped: the job's teardown
    // guard kills its processes and removes its files, and only then is the
    // slot (and with it the uid) returned to the pool.
    let resp = state.engine.execute(req, &slot).await;
    drop(slot);
    (status_for(&resp), Json(resp)).into_response()
}

fn status_for(resp: &ExecuteResponse) -> StatusCode {
    match resp.error.as_ref().map(|e| e.code) {
        None => StatusCode::OK,
        Some(RunnerErrorCode::BadRequest) => StatusCode::BAD_REQUEST,
        Some(RunnerErrorCode::UnsupportedLanguage) => StatusCode::UNPROCESSABLE_ENTITY,
        Some(RunnerErrorCode::Busy) => StatusCode::SERVICE_UNAVAILABLE,
        Some(RunnerErrorCode::Internal) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn refusal(
    status: StatusCode,
    job_id: &str,
    language: &str,
    code: RunnerErrorCode,
    message: impl Into<String>,
) -> Response {
    let mut body = ExecuteResponse::refused(job_id, language, RunnerError::new(code, message));
    body.runner_version = crate::RUNNER_VERSION.to_string();
    (status, Json(body)).into_response()
}

async fn require_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Response {
    if let Some(expected) = &state.engine.config().token {
        let presented = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if !presented.is_some_and(|p| token_matches(expected, p)) {
            let mut resp = refusal(
                StatusCode::UNAUTHORIZED,
                "",
                "",
                RunnerErrorCode::BadRequest,
                "missing or invalid bearer token",
            );
            resp.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                header::HeaderValue::from_static("Bearer"),
            );
            return resp;
        }
    }
    next.run(req).await
}

/// Compare tokens in time independent of *where* they differ, so the token
/// cannot be recovered byte by byte from response timings. (Only its length
/// is observable, which says nothing useful about a random secret.)
pub fn token_matches(expected: &str, presented: &str) -> bool {
    expected.as_bytes().ct_eq(presented.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_must_match_exactly() {
        assert!(token_matches("s3cret-token", "s3cret-token"));
        assert!(!token_matches("s3cret-token", "s3cret-tokeN"));
        assert!(!token_matches("s3cret-token", "s3cret-toke"));
        assert!(!token_matches("s3cret-token", "s3cret-token "));
        assert!(!token_matches("s3cret-token", ""));
    }

    #[test]
    fn refusals_map_to_meaningful_statuses() {
        let with = |code| ExecuteResponse::refused("j", "go", RunnerError::new(code, "x"));
        assert_eq!(
            status_for(&with(RunnerErrorCode::Busy)),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status_for(&with(RunnerErrorCode::BadRequest)),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_for(&with(RunnerErrorCode::UnsupportedLanguage)),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            status_for(&with(RunnerErrorCode::Internal)),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        let mut ok = with(RunnerErrorCode::Busy);
        ok.error = None;
        assert_eq!(status_for(&ok), StatusCode::OK);
    }
}
