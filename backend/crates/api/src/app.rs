//! The HTTP application: routes plus the middleware every request passes.
//!
//! Outermost to innermost:
//!
//! 1. `Cookie`/`Authorization` marked sensitive, so no log line can leak them.
//! 2. A request id (`x-request-id`, honoured if CloudFront/ALB sent one) that
//!    every log line of the request carries and the response echoes.
//! 3. Tracing, panic → JSON 500, a response timeout, compression.
//! 4. Security headers, with `Cache-Control: no-store` as the default — only
//!    the public content routes opt into caching, explicitly.
//! 5. The origin guard: the CloudFront shared secret (so the ALB cannot be
//!    used directly), and refusal of cross-origin writes.
//! 6. Per-route metrics.

use crate::config::normalize_origin;
use crate::error::ApiError;
use crate::routes;
use crate::security;
use crate::state::AppState;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use std::time::{Duration, Instant};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// Largest request body: a program is at most 128 KiB, a chat turn a few KiB.
const BODY_LIMIT: usize = 1024 * 1024;
/// Time to *start* a response — under CloudFront's 60 s origin read timeout,
/// so a slow request fails here, with our error body, not there. Streamed
/// bodies (AI replies) are not bound by it once their headers are sent.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(55);

pub fn build(state: AppState) -> Router {
    let request_id = HeaderName::from_static("x-request-id");

    Router::new()
        .route("/healthz", axum::routing::get(routes::health::healthz))
        .route("/readyz", axum::routing::get(routes::health::readyz))
        .nest("/api/v1", routes::api())
        .fallback(|| async { ApiError::NotFound("no such endpoint".into()) })
        .route_layer(middleware::from_fn(track))
        .layer(middleware::from_fn_with_state(state.clone(), origin_guard))
        .layer(
            ServiceBuilder::new()
                .layer(SetSensitiveRequestHeadersLayer::new([header::COOKIE, header::AUTHORIZATION]))
                .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http().make_span_with(|req: &Request| {
                        let id = req
                            .headers()
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("-");
                        tracing::info_span!("http", method = %req.method(), path = %req.uri().path(), request_id = %id)
                    }),
                )
                .layer(PropagateRequestIdLayer::new(request_id))
                .layer(CatchPanicLayer::custom(|_| {
                    ApiError::internal("handler panicked").into_response()
                }))
                .layer(CompressionLayer::new())
                .layer(RequestBodyLimitLayer::new(BODY_LIMIT))
                .layer(SetResponseHeaderLayer::if_not_present(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-store"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                // Innermost of the body-changing layers: a timeout answers
                // with an empty body of the router's own body type.
                .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, RESPONSE_TIMEOUT)),
        )
        .with_state(state)
}

fn is_safe(m: &Method) -> bool {
    matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Paths that must work without the CloudFront secret: the load balancer's
/// health checks reach the task directly.
fn is_health(path: &str) -> bool {
    path == "/healthz" || path == "/readyz"
}

/// Stripe calls the webhook server-to-server, with its own signature.
fn is_webhook(path: &str) -> bool {
    path == "/api/v1/billing/webhook"
}

async fn origin_guard(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if let Some(secret) = &state.cfg.origin_verify_secret {
        if !is_health(&path) {
            let ok = req
                .headers()
                .get("x-origin-verify")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| security::ct_eq(v.as_bytes(), secret.as_bytes()));
            if !ok {
                return ApiError::Forbidden("Requests must come through the site.".into())
                    .into_response();
            }
        }
    }

    // A browser always sends Origin on a cross-site write (and Referer
    // otherwise); a non-browser client sends neither and is not a CSRF vector.
    // Every value is checked: an allowed Origin beside a hostile one proves
    // nothing about who is asking.
    if !is_safe(req.method()) && !is_webhook(&path) {
        let mut claimed: Vec<String> = req
            .headers()
            .get_all(header::ORIGIN)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .filter(|o| *o != "null")
            .map(str::to_string)
            .collect();
        if claimed.is_empty() {
            claimed.extend(
                req.headers()
                    .get_all(header::REFERER)
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .filter_map(|r| normalize_origin(r).ok()),
            );
        }
        let refused = claimed.iter().find(|o| {
            !normalize_origin(o)
                .map(|n| state.cfg.allowed_origins.contains(&n))
                .unwrap_or(false)
        });
        if let Some(o) = refused {
            tracing::warn!(origin = %o, %path, "cross-origin write refused");
            return ApiError::Forbidden("Cross-origin request refused.".into()).into_response();
        }
    }
    next.run(req).await
}

/// Request count and latency per *route template* (`/api/v1/profiles/{pid}`),
/// never per concrete path — ids in labels would explode the series count.
async fn track(req: Request, next: Next) -> Response {
    let started = Instant::now();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".into());
    let method = req.method().as_str().to_string();
    let res = next.run(req).await;
    let status = res.status().as_u16().to_string();
    metrics::counter!("http_requests_total", "method" => method.clone(), "route" => route.clone(), "status" => status)
        .increment(1);
    metrics::histogram!("http_request_duration_seconds", "method" => method, "route" => route)
        .record(started.elapsed().as_secs_f64());
    res
}
