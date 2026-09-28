//! Problem content and traces.
//!
//! `/content/*` is public and identical for every viewer, so it is served with
//! `Cache-Control: public` and a strong ETag derived from the content version:
//! CloudFront answers almost all of it, and a browser revalidates with a 304.
//! Anything that depends on who is asking (premium sources, custom-input
//! traces) lives outside `/content` and is never cached.

use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, Client, Json, Path};
use crate::routes::common::{limit, premium_entitled, MINUTE};
use crate::state::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

/// Browsers revalidate after 5 minutes; the CDN holds an hour and may serve
/// stale for a day while it refetches. A deploy changes the ETag regardless.
const PUBLIC_CACHE: &str = "public, max-age=300, s-maxage=3600, stale-while-revalidate=86400";

fn cached_json(req_headers: &HeaderMap, etag: String, body: Bytes) -> Response {
    let etag = format!("\"{etag}\"");
    let fresh = req_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag || t.trim() == "*"));
    let mut res = if fresh {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        let mut r = Response::new(Body::from(body));
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        r
    };
    let h = res.headers_mut();
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(PUBLIC_CACHE),
    );
    if let Ok(v) = HeaderValue::from_str(&etag) {
        h.insert(header::ETAG, v);
    }
    res
}

fn private_json(body: Bytes) -> Response {
    let mut r = Response::new(Body::from(body));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    r
}

pub async fn catalog(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let body = Bytes::from((*state.content.catalog_json()).clone());
    cached_json(&headers, format!("cat-{}", state.content.version), body)
}

pub async fn guide(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let body = Bytes::from((*state.content.guide_json()).clone());
    cached_json(&headers, format!("guide-{}", state.content.version), body)
}

/// The public problem payload: premium problems come back `locked`, with
/// their statement (it is what search engines and the upgrade page show).
pub async fn public_problem(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let p = state
        .content
        .problem(&slug, false)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    let body = Bytes::from(serde_json::to_vec(&*p).map_err(ApiError::internal)?);
    Ok(cached_json(
        &headers,
        format!("p-{}-{slug}", state.content.version),
        body,
    ))
}

pub async fn public_trace(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let item = state
        .content
        .item(&slug)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    if state.content.is_premium(item.tier) {
        return Err(ApiError::PaymentRequired(
            "This animation is part of Pro.".into(),
        ));
    }
    let body = state.traces.default_trace(&slug).await?;
    Ok(cached_json(
        &headers,
        format!("t-{}-{slug}", state.content.version),
        body,
    ))
}

/// The problem as the caller may see it: full sources on a premium problem
/// when they are entitled. Signed-in users use this; guests use the public one.
pub async fn problem(
    State(state): State<AppState>,
    authed: Option<Authed>,
    Path(slug): Path<String>,
) -> ApiResult<Response> {
    let entitled = authed.as_ref().is_some_and(|a| premium_entitled(&a.user));
    let p = state
        .content
        .problem(&slug, entitled)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    Ok(private_json(Bytes::from(
        serde_json::to_vec(&*p).map_err(ApiError::internal)?,
    )))
}

/// Re-trace with edited input. Guests may use it (the animation is the
/// product's front door), under a tighter per-IP limit than accounts get.
pub async fn trace(
    State(state): State<AppState>,
    authed: Option<Authed>,
    client: Client,
    Path(slug): Path<String>,
    Json(req): Json<crate::dto::TraceRequest>,
) -> ApiResult<Response> {
    let item = state
        .content
        .item(&slug)
        .ok_or_else(|| ApiError::not_found("problem"))?;
    match &authed {
        Some(a) => limit(&state, &format!("trace:u:{}", a.user.id), 120, MINUTE).await?,
        None => limit(&state, &format!("trace:ip:{}", client.key()), 40, MINUTE).await?,
    }
    if state.content.is_premium(item.tier)
        && !authed.as_ref().is_some_and(|a| premium_entitled(&a.user))
    {
        return Err(ApiError::PaymentRequired(
            "This animation is part of Pro.".into(),
        ));
    }
    let input = match (req.fields, req.input) {
        (Some(fields), _) => state.traces.parse_fields(&slug, &fields)?,
        (None, Some(input)) => input,
        (None, None) => state
            .content
            .default_input(&slug)
            .cloned()
            .ok_or_else(|| ApiError::not_found("problem"))?,
    };
    let bytes = state.traces.trace(&slug, input).await?;
    Ok(private_json(bytes))
}
