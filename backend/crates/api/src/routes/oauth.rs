//! "Sign in with GitHub / Google": the authorization-code flow with PKCE.
//!
//! The round trip's state (CSRF `state`, PKCE verifier, where to land after)
//! rides in a short-lived HMAC-signed cookie rather than the database, so any
//! API task can finish a flow another task started.
//!
//! Account linking is the dangerous part. An identity is attached to an
//! existing account only when the provider vouches that the email is
//! verified; otherwise a stranger could register a provider account with
//! someone else's unverified address and walk into their progress.

use crate::error::{ApiError, ApiResult};
use crate::extract::{cookie, Client, Path, Query};
use crate::routes::common::session_cookie;
use crate::routes::health::dsa_colors;
use crate::security::{self, normalize_email};
use crate::state::AppState;
use crate::store::{accounts, practice};
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

const OAUTH_COOKIE: &str = "dsa_oauth";
const FLOW_TTL_SECS: i64 = 600;

#[derive(Serialize, Deserialize)]
struct Flow {
    provider: String,
    state: String,
    verifier: String,
    next: String,
    exp: i64,
}

struct ProviderCfg {
    authorize: &'static str,
    token: &'static str,
    scope: &'static str,
    client_id: String,
    client_secret: String,
}

fn provider(state: &AppState, name: &str) -> ApiResult<ProviderCfg> {
    let (c, authorize, token, scope) = match name {
        "github" => (
            state.cfg.github.as_ref(),
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
            "read:user user:email",
        ),
        "google" => (
            state.cfg.google.as_ref(),
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "openid email profile",
        ),
        _ => (None, "", "", ""),
    };
    let c = c.ok_or_else(|| ApiError::not_found("sign-in provider"))?;
    Ok(ProviderCfg {
        authorize,
        token,
        scope,
        client_id: c.client_id.clone(),
        client_secret: c.client_secret.clone(),
    })
}

fn redirect_uri(state: &AppState, provider: &str) -> String {
    state.public_link(&format!("/api/v1/auth/oauth/{provider}/callback"))
}

/// Only a same-site path is an acceptable place to land.
fn safe_next(next: Option<&str>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") && !n.contains('\\') => n.to_string(),
        _ => "/".to_string(),
    }
}

fn found(location: &str, cookies: Vec<HeaderValue>) -> Response {
    let mut h = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(location) {
        h.insert(header::LOCATION, v);
    }
    for c in cookies {
        h.append(header::SET_COOKIE, c);
    }
    (StatusCode::FOUND, h).into_response()
}

fn flow_cookie(state: &AppState, value: &str, max_age: i64) -> HeaderValue {
    let secure = if state.cfg.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{OAUTH_COOKIE}={value}; Path=/api/v1/auth/oauth; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    ))
    .expect("cookie is ASCII")
}

#[derive(Deserialize)]
pub struct StartQuery {
    next: Option<String>,
}

pub async fn start(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<StartQuery>,
) -> ApiResult<Response> {
    let p = provider(&state, &name)?;
    let flow = Flow {
        provider: name.clone(),
        state: security::random_token(),
        verifier: security::random_token(),
        next: safe_next(q.next.as_deref()),
        exp: OffsetDateTime::now_utc().unix_timestamp() + FLOW_TTL_SECS,
    };
    let mut url = url::Url::parse(p.authorize).map_err(ApiError::internal)?;
    url.query_pairs_mut()
        .append_pair("client_id", &p.client_id)
        .append_pair("redirect_uri", &redirect_uri(&state, &name))
        .append_pair("response_type", "code")
        .append_pair("scope", p.scope)
        .append_pair("state", &flow.state)
        .append_pair("code_challenge", &security::pkce_challenge(&flow.verifier))
        .append_pair("code_challenge_method", "S256");
    if name == "google" {
        url.query_pairs_mut()
            .append_pair("prompt", "select_account");
    }
    let signed = security::sign(
        &state.cfg.session_secret,
        "oauth",
        &serde_json::to_vec(&flow).map_err(ApiError::internal)?,
    );
    Ok(found(
        url.as_str(),
        vec![flow_cookie(&state, &signed, FLOW_TTL_SECS)],
    ))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Who the provider says this is.
struct Identity {
    subject: String,
    email: Option<String>,
    email_verified: bool,
    name: String,
}

pub async fn callback(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<CallbackQuery>,
    client: Client,
    headers: HeaderMap,
) -> Response {
    let clear = flow_cookie(&state, "", 0);
    let fail = |code: &str| found(&format!("/login?error={code}"), vec![clear.clone()]);

    if q.error.is_some() {
        return fail("oauth_denied");
    }
    let Some(flow) = cookie(&headers, OAUTH_COOKIE)
        .and_then(|v| security::unsign(&state.cfg.session_secret, "oauth", v))
        .and_then(|b| serde_json::from_slice::<Flow>(&b).ok())
    else {
        return fail("oauth_expired");
    };
    let state_ok = q
        .state
        .as_deref()
        .is_some_and(|s| security::ct_eq(s.as_bytes(), flow.state.as_bytes()));
    if !state_ok || flow.provider != name || flow.exp < OffsetDateTime::now_utc().unix_timestamp() {
        return fail("oauth_expired");
    }
    let Some(code) = q.code else {
        return fail("oauth_denied");
    };

    match finish(&state, &name, &code, &flow, &client).await {
        Ok((token, next)) => {
            metrics::counter!("dsa_logins_total", "result" => "ok").increment(1);
            found(
                &state.public_link(&next),
                vec![
                    clear.clone(),
                    session_cookie(&state, &token, state.cfg.session_ttl),
                ],
            )
        }
        Err(e) => {
            tracing::warn!(provider = %name, error = %e, "oauth sign-in failed");
            fail(match e {
                ApiError::Conflict(_) => "oauth_email_in_use",
                _ => "oauth_failed",
            })
        }
    }
}

async fn finish(
    state: &AppState,
    name: &str,
    code: &str,
    flow: &Flow,
    client: &Client,
) -> ApiResult<(String, String)> {
    let p = provider(state, name)?;
    let token: Value = state
        .http
        .post(p.token)
        .header(header::ACCEPT, "application/json")
        .form(&[
            ("client_id", p.client_id.as_str()),
            ("client_secret", p.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri(state, name).as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", flow.verifier.as_str()),
        ])
        .send()
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;
    let access = token["access_token"]
        .as_str()
        .ok_or_else(|| ApiError::internal(format!("{name}: no access token")))?;

    let id = match name {
        "github" => github_identity(state, access).await?,
        _ => google_identity(state, access).await?,
    };

    let user = resolve_account(state, name, &id).await?;
    let promote = user.verified()
        && !user.is_admin()
        && state.cfg.admin_emails.contains(&user.email_normalized);
    let user = accounts::record_login(&state.db, user.id, promote).await?;

    let session = security::random_token();
    accounts::create_session(
        &state.db,
        user.id,
        &session,
        state.cfg.session_ttl,
        client.user_agent.as_deref(),
        client.ip.as_deref(),
    )
    .await?;
    accounts::audit(
        &state.db,
        Some(user.id),
        "login",
        client.ip.as_deref(),
        json!({ "method": name }),
    )
    .await;
    Ok((session, flow.next.clone()))
}

async fn github_identity(state: &AppState, access: &str) -> ApiResult<Identity> {
    let get = |url: &'static str| {
        state
            .http
            .get(url)
            .bearer_auth(access)
            .header(header::USER_AGENT, "dsa-visualized")
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
    };
    let me: Value = get("https://api.github.com/user")
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;
    let emails: Value = get("https://api.github.com/user/emails")
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;
    let primary = emails
        .as_array()
        .and_then(|a| {
            a.iter().find(|e| {
                e["primary"].as_bool() == Some(true) && e["verified"].as_bool() == Some(true)
            })
        })
        .and_then(|e| e["email"].as_str())
        .map(str::to_string);
    let subject = me["id"]
        .as_i64()
        .map(|i| i.to_string())
        .ok_or_else(|| ApiError::internal("github: no user id"))?;
    Ok(Identity {
        subject,
        email_verified: primary.is_some(),
        email: primary,
        name: me["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or(me["login"].as_str())
            .unwrap_or("GitHub user")
            .to_string(),
    })
}

async fn google_identity(state: &AppState, access: &str) -> ApiResult<Identity> {
    let info: Value = state
        .http
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(access)
        .send()
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;
    Ok(Identity {
        subject: info["sub"]
            .as_str()
            .ok_or_else(|| ApiError::internal("google: no subject"))?
            .to_string(),
        email: info["email"].as_str().map(str::to_string),
        email_verified: info["email_verified"].as_bool() == Some(true),
        name: info["name"].as_str().unwrap_or("Google user").to_string(),
    })
}

/// Identity → account: a known identity signs in; a verified email that
/// matches an account links to it; anything else becomes a new account.
async fn resolve_account(
    state: &AppState,
    provider: &str,
    id: &Identity,
) -> ApiResult<accounts::UserRow> {
    if let Some(u) = accounts::user_by_identity(&state.db, provider, &id.subject).await? {
        return Ok(u);
    }
    let email = id.email.clone().ok_or_else(|| {
        ApiError::BadRequest("The provider did not share an email address.".into())
    })?;
    let email_n = normalize_email(&email);

    if let Some(existing) = accounts::user_by_email(&state.db, &email_n).await? {
        if !id.email_verified {
            return Err(ApiError::Conflict(
                "email belongs to an existing account".into(),
            ));
        }
        accounts::link_identity(&state.db, provider, &id.subject, existing.id, Some(&email))
            .await?;
        accounts::mark_verified(&state.db, existing.id).await?;
        return accounts::user_by_id(&state.db, existing.id)
            .await?
            .ok_or_else(|| ApiError::internal("linked user vanished"));
    }

    let name: String = id.name.trim().chars().take(48).collect();
    let name = if name.is_empty() {
        "New learner".to_string()
    } else {
        name
    };
    let mut tx = state.db.begin().await?;
    let user = accounts::insert_user(
        &mut tx,
        accounts::NewUser {
            email: &email,
            email_normalized: &email_n,
            password_hash: None,
            display_name: &name,
            timezone: "UTC",
            verified: id.email_verified,
            role: "user",
        },
    )
    .await?;
    let color = dsa_colors()[(user.id.as_u128() % dsa_colors().len() as u128) as usize];
    practice::insert_profile(
        &mut tx,
        user.id,
        &name.chars().take(32).collect::<String>(),
        "🎓",
        color,
    )
    .await?;
    accounts::link_identity(&mut *tx, provider, &id.subject, user.id, Some(&email)).await?;
    tx.commit().await?;
    metrics::counter!("dsa_signups_total", "method" => provider.to_string()).increment(1);
    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_only_accepts_same_site_paths() {
        assert_eq!(safe_next(Some("/problems/two-sum")), "/problems/two-sum");
        assert_eq!(safe_next(Some("//evil.example")), "/");
        assert_eq!(safe_next(Some("https://evil.example")), "/");
        assert_eq!(safe_next(Some("/\\evil.example")), "/");
        assert_eq!(safe_next(None), "/");
    }
}
