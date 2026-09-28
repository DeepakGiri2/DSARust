//! The signed-in account: read, update, export, delete.

use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, Client, Json};
use crate::routes::common::{clean_name, clear_session_cookie, user_dto, with_cookie};
use crate::security::{self, normalize_email};
use crate::state::AppState;
use crate::store::{accounts, practice};
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub async fn get(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<dto::User>> {
    Ok(axum::Json(user_dto(&state, &authed.user).await?))
}

pub async fn update(
    State(state): State<AppState>,
    authed: Authed,
    Json(req): Json<dto::UpdateMeRequest>,
) -> ApiResult<axum::Json<dto::User>> {
    let name = match req.display_name.as_deref() {
        Some(n) => Some(clean_name("display_name", n, 48)?),
        None => None,
    };
    let tz = match req.timezone.as_deref().map(str::trim) {
        Some(tz) if accounts::is_valid_timezone(&state.db, tz).await? => Some(tz.to_string()),
        Some(_) => return Err(ApiError::field("timezone", "Unknown time zone.")),
        None => None,
    };
    let user =
        accounts::update_me(&state.db, authed.user.id, name.as_deref(), tz.as_deref()).await?;
    Ok(axum::Json(user_dto(&state, &user).await?))
}

/// Delete the account and everything it owns (the foreign keys cascade).
/// Typing the address is the confirmation; the password is proof of
/// possession for accounts that have one.
pub async fn delete(
    State(state): State<AppState>,
    authed: Authed,
    client: Client,
    Json(req): Json<dto::DeleteAccountRequest>,
) -> ApiResult<Response> {
    if normalize_email(&req.confirm_email) != authed.user.email_normalized {
        return Err(ApiError::field(
            "confirm_email",
            "Type your account's email address to confirm.",
        ));
    }
    if authed.user.password_hash.is_some() {
        let pw = req.password.unwrap_or_default();
        if !security::verify_password_async(pw, authed.user.password_hash.clone()).await {
            return Err(ApiError::field("password", "That isn't your password."));
        }
    }
    if let Some(customer) = &authed.user.stripe_customer_id {
        // Best effort: an account that cannot be deleted because Stripe is
        // down would be worse than a subscription cancelled by hand.
        if let Err(e) = crate::routes::billing::cancel_all(&state, customer).await {
            tracing::error!(error = %e, user = %authed.user.id, "could not cancel subscriptions on account deletion");
        }
    }
    accounts::delete_user(&state.db, authed.user.id).await?;
    accounts::audit(
        &state.db,
        None,
        "account_deleted",
        client.ip.as_deref(),
        json!({ "user_id": authed.user.id }),
    )
    .await;
    Ok((
        StatusCode::NO_CONTENT,
        with_cookie(clear_session_cookie(&state)),
    )
        .into_response())
}

/// Every row the account owns, as one JSON document (GDPR access request).
pub async fn export(State(state): State<AppState>, authed: Authed) -> ApiResult<Response> {
    let db = &state.db;
    let uid = authed.user.id;
    let mut profiles_out = Vec::new();
    for p in practice::profiles(db, uid).await? {
        let settings = practice::settings(db, p.id).await?;
        let progress = practice::snapshot(db, p.id).await?;
        let playlists = practice::playlists(db, p.id).await?;
        let drafts: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, String)>(
            "SELECT slug, lang, code FROM drafts WHERE profile_id = $1 ORDER BY slug, lang",
        )
        .bind(p.id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|(slug, lang, code)| json!({ "slug": slug, "lang": lang, "code": code }))
        .collect();
        let submissions: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, String, String, time::OffsetDateTime, String)>(
            "SELECT slug, lang, kind, status, created_at, code FROM submissions
             WHERE profile_id = $1 ORDER BY created_at",
        )
        .bind(p.id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|(slug, lang, kind, status, at, code)| {
            json!({
                "slug": slug, "lang": lang, "kind": kind, "status": status,
                "created_at": at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                "code": code,
            })
        })
        .collect();
        profiles_out.push(json!({
            "profile": p,
            "settings": settings,
            "progress": progress,
            "playlists": playlists,
            "drafts": drafts,
            "submissions": submissions,
        }));
    }
    let doc = json!({
        "exported_at": time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        "account": user_dto(&state, &authed.user).await?,
        "profiles": profiles_out,
    });
    let body = serde_json::to_vec_pretty(&doc).map_err(ApiError::internal)?;
    let mut res = Response::new(axum::body::Body::from(body));
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"dsa-visualized-export.json\""),
    );
    Ok(res)
}
