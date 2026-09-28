//! Pieces several route modules share: cookies, the `SessionInfo` payload,
//! entitlements, the profile guard and small validators.

use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, SESSION_COOKIE};
use crate::state::AppState;
use crate::store::{accounts, metering, practice};
use axum::http::{header, HeaderMap, HeaderValue};
use std::time::Duration;
use uuid::Uuid;

/// The session cookie: HttpOnly (no script can read it), SameSite=Lax (not
/// sent on cross-site subrequests), Secure whenever the site is https.
pub fn session_cookie(state: &AppState, token: &str, max_age: Duration) -> HeaderValue {
    let secure = if state.cfg.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{secure}",
        max_age.as_secs()
    ))
    .expect("cookie is ASCII")
}

pub fn clear_session_cookie(state: &AppState) -> HeaderValue {
    session_cookie(state, "", Duration::ZERO)
}

pub fn with_cookie(cookie: HeaderValue) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(header::SET_COOKIE, cookie);
    h
}

pub fn premium_entitled(user: &accounts::UserRow) -> bool {
    user.is_pro() || user.is_admin()
}

pub async fn entitlements(
    state: &AppState,
    user: &accounts::UserRow,
) -> ApiResult<dto::Entitlements> {
    let p = &state.cfg.policy;
    let pro = premium_entitled(user);
    Ok(dto::Entitlements {
        plan: user.plan.clone(),
        premium_content: pro,
        ai_daily_limit: if pro { p.ai_pro_daily } else { p.ai_free_daily },
        ai_used_today: metering::ai_used_today(&state.db, user.id).await?.max(0) as u32,
        runs_per_minute: if pro {
            p.runs_pro_per_min
        } else {
            p.runs_free_per_min
        },
    })
}

pub async fn user_dto(state: &AppState, user: &accounts::UserRow) -> ApiResult<dto::User> {
    Ok(dto::User {
        id: user.id,
        email: user.email.clone(),
        email_verified: user.verified(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
        plan: user.plan.clone(),
        plan_renews_at: user.plan_renews_at,
        timezone: user.timezone.clone(),
        has_password: user.password_hash.is_some(),
        oauth_providers: accounts::oauth_providers(&state.db, user.id).await?,
        created_at: user.created_at,
    })
}

/// Everything the SPA needs after sign-in, in one response.
pub async fn session_info(
    state: &AppState,
    user: &accounts::UserRow,
    session_id: Uuid,
) -> ApiResult<dto::SessionInfo> {
    Ok(dto::SessionInfo {
        user: user_dto(state, user).await?,
        csrf_token: crate::security::csrf_token(&state.cfg.session_secret, &session_id),
        entitlements: entitlements(state, user).await?,
        profiles: practice::profiles(&state.db, user.id).await?,
    })
}

/// 404 unless the profile is the caller's — never 403, so ids cannot be
/// probed across accounts.
pub async fn own_profile(state: &AppState, authed: &Authed, pid: Uuid) -> ApiResult<()> {
    if practice::owns_profile(&state.db, authed.user.id, pid).await? {
        let db = state.db.clone();
        tokio::spawn(async move {
            let _ = practice::touch_profile(&db, pid).await;
        });
        Ok(())
    } else {
        Err(ApiError::not_found("profile"))
    }
}

/// Writes may only name problems the catalogue lists.
pub fn known_slug(state: &AppState, slug: &str) -> ApiResult<()> {
    if state.content.has(slug) {
        Ok(())
    } else {
        Err(ApiError::not_found("problem"))
    }
}

/// Trim and bound a display string; refuse control characters, which render
/// as nothing and let two names look identical.
pub fn clean_name(field: &str, raw: &str, max: usize) -> ApiResult<String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(ApiError::field(field, "This can't be empty."));
    }
    if s.chars().count() > max {
        return Err(ApiError::field(
            field,
            format!("Use at most {max} characters."),
        ));
    }
    if s.chars().any(|c| c.is_control()) {
        return Err(ApiError::field(
            field,
            "That contains characters that can't be shown.",
        ));
    }
    Ok(s.to_string())
}

/// Apply a rate limit, answering 429 with the window's reset time.
pub async fn limit(state: &AppState, key: &str, max: u32, window: Duration) -> ApiResult<()> {
    let d = state.limiter.hit(key, max, window).await;
    if d.allowed {
        Ok(())
    } else {
        metrics::counter!("dsa_rate_limited_total").increment(1);
        Err(ApiError::RateLimited {
            retry_after_secs: d.retry_after,
        })
    }
}

pub const MINUTE: Duration = Duration::from_secs(60);
pub const HOUR: Duration = Duration::from_secs(3600);
