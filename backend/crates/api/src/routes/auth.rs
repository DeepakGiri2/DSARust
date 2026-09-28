//! Email + password accounts, sessions and the email-link flows.
//!
//! Defences, in the order an attacker meets them: per-IP and per-email rate
//! limits (Redis, so they hold across tasks), a per-account lockout after ten
//! misses (Postgres, so it holds even if Redis is down), Argon2id hashes,
//! identical timing and wording for "no such account" and "wrong password",
//! and 202-always on password reset so it cannot enumerate accounts.

use crate::dto;
use crate::error::{is_unique_violation, ApiError, ApiResult};
use crate::extract::{Authed, Client, Json, Path};
use crate::mail;
use crate::routes::common::{
    clean_name, clear_session_cookie, limit, session_cookie, session_info, with_cookie, HOUR,
};
use crate::routes::health::dsa_colors;
use crate::security::{self, email_problem, normalize_email, password_problem};
use crate::state::AppState;
use crate::store::{accounts, practice};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

const VERIFY_TTL: Duration = Duration::from_secs(48 * 3600);
const RESET_TTL: Duration = Duration::from_secs(3600);

/// Start a session for `user` and build the sign-in response.
pub async fn start_session(
    state: &AppState,
    user: &accounts::UserRow,
    client: &Client,
    status: StatusCode,
) -> ApiResult<axum::response::Response> {
    let token = security::random_token();
    let (session_id, _) = accounts::create_session(
        &state.db,
        user.id,
        &token,
        state.cfg.session_ttl,
        client.user_agent.as_deref(),
        client.ip.as_deref(),
    )
    .await?;
    let info = session_info(state, user, session_id).await?;
    let cookie = session_cookie(state, &token, state.cfg.session_ttl);
    Ok((status, with_cookie(cookie), axum::Json(info)).into_response())
}

/// Admin rights follow a verified address only — otherwise whoever registers
/// an `ADMIN_EMAILS` address first would get them.
fn should_promote(state: &AppState, user: &accounts::UserRow) -> bool {
    user.verified() && !user.is_admin() && state.cfg.admin_emails.contains(&user.email_normalized)
}

pub async fn send_verification(state: &AppState, user: &accounts::UserRow) -> ApiResult<()> {
    let token = security::random_token();
    accounts::create_auth_token(&state.db, user.id, "verify_email", &token, VERIFY_TTL).await?;
    let link = state.public_link(&format!("/verify-email?token={token}"));
    let email = mail::verification(&user.email, &user.display_name, &link);
    let st = state.clone();
    tokio::spawn(async move {
        if let Err(e) = st.mailer.send(email).await {
            tracing::warn!(error = %e, "verification email failed");
        }
    });
    Ok(())
}

// ── signup / login / logout / session ──────────────────────────────────────

pub async fn signup(
    State(state): State<AppState>,
    client: Client,
    Json(req): Json<dto::SignupRequest>,
) -> ApiResult<impl IntoResponse> {
    if !state.cfg.policy.signup_enabled {
        return Err(ApiError::Forbidden("Sign-ups are closed right now.".into()));
    }
    limit(&state, &format!("signup:{}", client.key()), 10, HOUR).await?;

    let mut fields = std::collections::BTreeMap::new();
    if let Some(p) = email_problem(&req.email) {
        fields.insert("email".to_string(), p.to_string());
    }
    let name = match clean_name("display_name", &req.display_name, 48) {
        Ok(n) => Some(n),
        Err(ApiError::Validation { fields: f, .. }) => {
            fields.extend(f);
            None
        }
        Err(e) => return Err(e),
    };
    if let Some(p) = password_problem(&req.password, &req.email) {
        fields.insert("password".to_string(), p);
    }
    if !fields.is_empty() {
        return Err(ApiError::fields(fields));
    }
    let name = name.expect("validated");
    let email = req.email.trim().to_string();
    let email_n = normalize_email(&email);
    let tz = match req
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        Some(tz) if accounts::is_valid_timezone(&state.db, tz).await? => tz.to_string(),
        _ => "UTC".to_string(),
    };

    let hash = security::hash_password_async(req.password).await?;
    let mut tx = state.db.begin().await?;
    let user = match accounts::insert_user(
        &mut tx,
        accounts::NewUser {
            email: &email,
            email_normalized: &email_n,
            password_hash: Some(&hash),
            display_name: &name,
            timezone: &tz,
            verified: false,
            role: "user",
        },
    )
    .await
    {
        Ok(u) => u,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::Conflict(
                "An account with this email already exists. Sign in instead?".into(),
            ))
        }
        Err(e) => return Err(e.into()),
    };
    // The account's first profile, named after its owner — a single-profile
    // account never sees the picker.
    let color = dsa_colors()[(user.id.as_u128() % dsa_colors().len() as u128) as usize];
    let profile_name: String = name.chars().take(32).collect();
    practice::insert_profile(&mut tx, user.id, &profile_name, "🎓", color).await?;
    tx.commit().await?;

    accounts::audit(
        &state.db,
        Some(user.id),
        "signup",
        client.ip.as_deref(),
        json!({}),
    )
    .await;
    metrics::counter!("dsa_signups_total", "method" => "password").increment(1);
    send_verification(&state, &user).await?;
    start_session(&state, &user, &client, StatusCode::CREATED).await
}

pub async fn login(
    State(state): State<AppState>,
    client: Client,
    Json(req): Json<dto::LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let email_n = normalize_email(&req.email);
    limit(
        &state,
        &format!("login:ip:{}", client.key()),
        30,
        Duration::from_secs(600),
    )
    .await?;
    limit(
        &state,
        &format!("login:email:{email_n}"),
        10,
        Duration::from_secs(900),
    )
    .await?;
    if req.password.len() > security::PASSWORD_MAX * 4 {
        return Err(ApiError::Unauthorized("Wrong email or password.".into()));
    }

    let user = accounts::user_by_email(&state.db, &email_n).await?;
    if let Some(u) = &user {
        if let Some(until) = u.locked_until.filter(|t| *t > OffsetDateTime::now_utc()) {
            return Err(ApiError::AccountLocked {
                retry_after_secs: (until - OffsetDateTime::now_utc()).whole_seconds().max(1) as u64,
            });
        }
    }
    let ok = security::verify_password_async(
        req.password,
        user.as_ref().and_then(|u| u.password_hash.clone()),
    )
    .await;
    let Some(user) = user.filter(|_| ok) else {
        if let Some(u) = accounts::user_by_email(&state.db, &email_n).await? {
            let locked = accounts::record_failed_login(&state.db, u.id).await?;
            accounts::audit(
                &state.db,
                Some(u.id),
                "login_failed",
                client.ip.as_deref(),
                json!({}),
            )
            .await;
            if let Some(until) = locked.filter(|t| *t > OffsetDateTime::now_utc()) {
                return Err(ApiError::AccountLocked {
                    retry_after_secs: (until - OffsetDateTime::now_utc()).whole_seconds().max(1)
                        as u64,
                });
            }
        }
        metrics::counter!("dsa_logins_total", "result" => "failed").increment(1);
        return Err(ApiError::Unauthorized("Wrong email or password.".into()));
    };
    let promote = should_promote(&state, &user);
    let user = accounts::record_login(&state.db, user.id, promote).await?;
    accounts::audit(
        &state.db,
        Some(user.id),
        "login",
        client.ip.as_deref(),
        json!({ "method": "password" }),
    )
    .await;
    metrics::counter!("dsa_logins_total", "result" => "ok").increment(1);
    start_session(&state, &user, &client, StatusCode::OK).await
}

pub async fn logout(State(state): State<AppState>, authed: Authed) -> ApiResult<impl IntoResponse> {
    accounts::revoke_session(&state.db, authed.user.id, authed.session_id).await?;
    Ok((
        StatusCode::NO_CONTENT,
        with_cookie(clear_session_cookie(&state)),
    ))
}

pub async fn session(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<dto::SessionInfo>> {
    Ok(axum::Json(
        session_info(&state, &authed.user, authed.session_id).await?,
    ))
}

// ── email verification ──────────────────────────────────────────────────────

pub async fn verify_email(
    State(state): State<AppState>,
    client: Client,
    Json(req): Json<dto::TokenRequest>,
) -> ApiResult<StatusCode> {
    limit(&state, &format!("verify:{}", client.key()), 30, HOUR).await?;
    let mut tx = state.db.begin().await?;
    let user_id = accounts::consume_auth_token(&mut tx, "verify_email", req.token.trim())
        .await?
        .ok_or_else(|| ApiError::BadRequest("This link has expired or was already used.".into()))?;
    accounts::mark_verified(&mut *tx, user_id).await?;
    tx.commit().await?;
    if let Some(user) = accounts::user_by_id(&state.db, user_id).await? {
        if should_promote(&state, &user) {
            accounts::record_login(&state.db, user.id, true).await?;
        }
    }
    accounts::audit(
        &state.db,
        Some(user_id),
        "email_verified",
        client.ip.as_deref(),
        json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn resend_verification(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    if authed.user.verified() {
        return Ok(StatusCode::NO_CONTENT);
    }
    limit(&state, &format!("resend:{}", authed.user.id), 3, HOUR).await?;
    send_verification(&state, &authed.user).await?;
    Ok(StatusCode::ACCEPTED)
}

// ── passwords ───────────────────────────────────────────────────────────────

pub async fn forgot_password(
    State(state): State<AppState>,
    client: Client,
    Json(req): Json<dto::EmailRequest>,
) -> ApiResult<StatusCode> {
    let email_n = normalize_email(&req.email);
    limit(&state, &format!("forgot:ip:{}", client.key()), 10, HOUR).await?;
    // Over the per-address limit the answer is still 202: a different status
    // would tell the caller the address exists.
    let under = state
        .limiter
        .hit(&format!("forgot:email:{email_n}"), 3, HOUR)
        .await
        .allowed;
    if under {
        if let Some(user) = accounts::user_by_email(&state.db, &email_n).await? {
            let token = security::random_token();
            accounts::create_auth_token(&state.db, user.id, "reset_password", &token, RESET_TTL)
                .await?;
            let link = state.public_link(&format!("/reset-password?token={token}"));
            let email = mail::password_reset(&user.email, &user.display_name, &link);
            let st = state.clone();
            tokio::spawn(async move {
                if let Err(e) = st.mailer.send(email).await {
                    tracing::warn!(error = %e, "reset email failed");
                }
            });
            accounts::audit(
                &state.db,
                Some(user.id),
                "password_reset_requested",
                client.ip.as_deref(),
                json!({}),
            )
            .await;
        }
    }
    Ok(StatusCode::ACCEPTED)
}

pub async fn reset_password(
    State(state): State<AppState>,
    client: Client,
    Json(req): Json<dto::ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    limit(&state, &format!("reset:{}", client.key()), 20, HOUR).await?;
    let mut tx = state.db.begin().await?;
    let user_id = accounts::consume_auth_token(&mut tx, "reset_password", req.token.trim())
        .await?
        .ok_or_else(|| ApiError::BadRequest("This link has expired or was already used.".into()))?;
    let user = accounts::user_by_id(&mut *tx, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("account"))?;
    if let Some(p) = password_problem(&req.password, &user.email) {
        // Rolling back un-spends the token, so a weak first choice does not
        // cost the user their link.
        return Err(ApiError::field("password", p));
    }
    let hash = security::hash_password_async(req.password).await?;
    accounts::set_password(&mut *tx, user_id, &hash).await?;
    accounts::void_auth_tokens(&mut *tx, user_id, "reset_password").await?;
    // Whoever knew the old password is signed out everywhere.
    accounts::revoke_all_sessions(&mut *tx, user_id, None).await?;
    // Receiving the email proves ownership of the address.
    accounts::mark_verified(&mut *tx, user_id).await?;
    tx.commit().await?;
    accounts::audit(
        &state.db,
        Some(user_id),
        "password_reset",
        client.ip.as_deref(),
        json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_password(
    State(state): State<AppState>,
    authed: Authed,
    client: Client,
    Json(req): Json<dto::ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    limit(&state, &format!("pwchange:{}", authed.user.id), 10, HOUR).await?;
    if authed.user.password_hash.is_some()
        && !security::verify_password_async(req.current_password, authed.user.password_hash.clone())
            .await
    {
        return Err(ApiError::field(
            "current_password",
            "That isn't your current password.",
        ));
    }
    if let Some(p) = password_problem(&req.new_password, &authed.user.email) {
        return Err(ApiError::field("new_password", p));
    }
    let hash = security::hash_password_async(req.new_password).await?;
    let mut tx = state.db.begin().await?;
    accounts::set_password(&mut *tx, authed.user.id, &hash).await?;
    accounts::revoke_all_sessions(&mut *tx, authed.user.id, Some(authed.session_id)).await?;
    tx.commit().await?;
    accounts::audit(
        &state.db,
        Some(authed.user.id),
        "password_changed",
        client.ip.as_deref(),
        json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// ── sessions ────────────────────────────────────────────────────────────────

pub async fn list_sessions(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<Vec<dto::SessionRow>>> {
    let rows = accounts::list_sessions(&state.db, authed.user.id).await?;
    Ok(axum::Json(
        rows.into_iter()
            .map(|r| dto::SessionRow {
                current: r.id == authed.session_id,
                id: r.id,
                created_at: r.created_at,
                last_seen_at: r.last_seen_at,
                expires_at: r.expires_at,
                user_agent: r.user_agent,
                ip: r.ip,
            })
            .collect(),
    ))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    authed: Authed,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if accounts::revoke_session(&state.db, authed.user.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("session"))
    }
}

pub async fn revoke_other_sessions(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    accounts::revoke_all_sessions(&state.db, authed.user.id, Some(authed.session_id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
