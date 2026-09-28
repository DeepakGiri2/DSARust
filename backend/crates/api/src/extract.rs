//! Request extractors: who is calling, from where, and with what body.
//!
//! Authentication and CSRF are one step here, not two. `Authed` resolves the
//! session cookie *and*, for any state-changing method, requires the
//! session-bound `X-CSRF-Token`. A handler that takes `Authed` therefore
//! cannot forget CSRF protection — there is no separate middleware to leave
//! off a route.

use crate::error::ApiError;
use crate::security;
use crate::state::AppState;
use crate::store::accounts::{self, SessionUser, UserRow};
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{ConnectInfo, FromRequest, FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, Method};
use std::net::SocketAddr;
use time::OffsetDateTime;
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "dsa_session";
pub const CSRF_HEADER: &str = "x-csrf-token";

/// A signed-in caller.
#[derive(Clone, Debug)]
pub struct Authed {
    pub session_id: Uuid,
    pub user: UserRow,
    pub token: String,
}

impl Authed {
    pub fn csrf_token(&self, secret: &[u8]) -> String {
        security::csrf_token(secret, &self.session_id)
    }
}

/// Read a cookie from the `Cookie` header without pulling in a cookie jar.
pub fn cookie<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

fn is_safe(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Sessions slide forward on use; writing that on every request would be a
/// write per request, so it happens at most this often.
const TOUCH_EVERY: time::Duration = time::Duration::minutes(10);

async fn resolve(parts: &mut Parts, state: &AppState) -> Result<Option<Authed>, ApiError> {
    // Cache per request: a handler may ask for the caller more than once.
    if let Some(cached) = parts.extensions.get::<Option<Authed>>() {
        return Ok(cached.clone());
    }
    let Some(token) = cookie(&parts.headers, SESSION_COOKIE).map(str::to_string) else {
        parts.extensions.insert(None::<Authed>);
        return Ok(None);
    };
    // A token is 43 base64url characters; anything else cannot be valid and
    // is not worth a database round trip.
    if token.len() != 43 {
        parts.extensions.insert(None::<Authed>);
        return Ok(None);
    }
    let found: Option<SessionUser> = accounts::session_by_token(&state.db, &token).await?;
    let authed = found.map(|s| {
        if OffsetDateTime::now_utc() - s.session_seen_at > TOUCH_EVERY {
            let (db, id, ttl) = (state.db.clone(), s.session_id, state.cfg.session_ttl);
            let ip = client_ip(parts, state.cfg.trusted_proxy_hops);
            tokio::spawn(async move {
                if let Err(e) = accounts::touch_session(&db, id, ttl, ip.as_deref()).await {
                    tracing::warn!(error = %e, "session touch failed");
                }
            });
        }
        Authed {
            session_id: s.session_id,
            user: s.user,
            token,
        }
    });
    parts.extensions.insert(authed.clone());
    Ok(authed)
}

fn check_csrf(parts: &Parts, state: &AppState, authed: &Authed) -> Result<(), ApiError> {
    if is_safe(&parts.method) {
        return Ok(());
    }
    let presented = parts
        .headers
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Csrf)?;
    if security::verify_csrf(&state.cfg.session_secret, &authed.session_id, presented) {
        Ok(())
    } else {
        Err(ApiError::Csrf)
    }
}

impl FromRequestParts<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let authed = resolve(parts, state)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("Sign in to continue.".into()))?;
        check_csrf(parts, state, &authed)?;
        Ok(authed)
    }
}

/// `Option<Authed>`: guests are fine. A cookie that fails CSRF on a write is
/// treated as a guest rather than an error, so an optional-auth write (a
/// custom trace) never acts *as* the user without proof it is the user.
impl OptionalFromRequestParts<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        let Some(authed) = resolve(parts, state).await? else {
            return Ok(None);
        };
        Ok(check_csrf(parts, state, &authed).ok().map(|_| authed))
    }
}

/// The caller's address: the socket peer, or — behind proxies — the entry
/// `TRUSTED_PROXY_HOPS` from the right of `X-Forwarded-For`. Everything to the
/// left of that is client-controlled and ignored.
pub fn client_ip(parts: &Parts, hops: usize) -> Option<String> {
    if hops > 0 {
        let chain: Vec<&str> = parts
            .headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if chain.len() >= hops {
            return Some(chain[chain.len() - hops].to_string());
        }
        if let Some(first) = chain.first() {
            return Some(first.to_string());
        }
    }
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_string())
}

#[derive(Clone, Debug)]
pub struct Client {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl Client {
    /// The rate-limit key for anonymous callers.
    pub fn key(&self) -> String {
        self.ip.clone().unwrap_or_else(|| "unknown".into())
    }
}

impl FromRequestParts<AppState> for Client {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Client {
            ip: client_ip(parts, state.cfg.trusted_proxy_hops),
            user_agent: parts
                .headers
                .get(header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.chars().take(400).collect()),
        })
    }
}

// ── bodies and paths with API-shaped rejections ─────────────────────────────

/// `axum::Json` whose rejections are `ApiError`s (JSON bodies, right codes).
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        ApiError::BadRequest(match r {
            JsonRejection::MissingJsonContentType(_) => {
                "Send a JSON body with Content-Type: application/json.".into()
            }
            other => other.body_text(),
        })
    }
}

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct Path<T>(pub T);

impl From<PathRejection> for ApiError {
    fn from(_: PathRejection) -> Self {
        // A malformed id is simply a thing that does not exist.
        ApiError::NotFound("not found".into())
    }
}

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub struct Query<T>(pub T);

impl From<QueryRejection> for ApiError {
    fn from(r: QueryRejection) -> Self {
        ApiError::BadRequest(r.body_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    fn parts(xff: &[&str], cookie_header: Option<&str>) -> Parts {
        let mut b = Request::builder().uri("/");
        for v in xff {
            b = b.header("x-forwarded-for", *v);
        }
        if let Some(c) = cookie_header {
            b = b.header(header::COOKIE, c);
        }
        b.body(()).unwrap().into_parts().0
    }

    #[test]
    fn the_client_ip_ignores_spoofed_left_entries() {
        // CloudFront appends the viewer, the ALB appends CloudFront.
        let p = parts(&["6.6.6.6, 1.2.3.4, 130.176.0.1"], None);
        assert_eq!(client_ip(&p, 2).as_deref(), Some("1.2.3.4"));
        assert_eq!(client_ip(&p, 1).as_deref(), Some("130.176.0.1"));
        // Split across headers is still one chain.
        let p = parts(&["1.2.3.4", "130.176.0.1"], None);
        assert_eq!(client_ip(&p, 2).as_deref(), Some("1.2.3.4"));
        // No proxies trusted: the header is ignored entirely.
        assert_eq!(client_ip(&p, 0), None);
    }

    #[test]
    fn cookies_are_found_among_others() {
        let p = parts(&[], Some("theme=dark; dsa_session=abc123; other=1"));
        assert_eq!(cookie(&p.headers, SESSION_COOKIE), Some("abc123"));
        assert_eq!(cookie(&p.headers, "missing"), None);
    }
}
