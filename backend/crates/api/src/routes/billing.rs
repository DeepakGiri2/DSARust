//! Pro subscriptions through Stripe: Checkout to subscribe, the customer
//! portal to manage, and a webhook that keeps `users.plan` in step.
//!
//! The webhook is the only thing that changes a plan. A successful redirect
//! back from Checkout proves nothing (anyone can type the URL); a signed event
//! from Stripe does. Events are verified (HMAC-SHA256 over `t.payload`, five
//! minute tolerance), deduplicated by id, and applied in one transaction.

use crate::config::StripeConfig;
use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::{Authed, Json};
use crate::security::ct_eq;
use crate::state::AppState;
use crate::store::{accounts, metering};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use time::OffsetDateTime;

const API: &str = "https://api.stripe.com/v1";

fn stripe(state: &AppState) -> ApiResult<&StripeConfig> {
    state
        .cfg
        .stripe
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("billing is not enabled on this server".into()))
}

async fn post_form(
    state: &AppState,
    path: &str,
    form: &[(&str, String)],
    idempotency: Option<&str>,
) -> ApiResult<Value> {
    let key = &stripe(state)?.secret_key;
    let mut req = state
        .http
        .post(format!("{API}{path}"))
        .basic_auth(key, Some(""))
        .form(form);
    if let Some(k) = idempotency {
        req = req.header("Idempotency-Key", k);
    }
    let resp = req.send().await.map_err(ApiError::internal)?;
    let status = resp.status();
    let body: Value = resp.json().await.map_err(ApiError::internal)?;
    if !status.is_success() {
        return Err(ApiError::internal(format!(
            "stripe {path} {status}: {}",
            body["error"]["message"].as_str().unwrap_or("unknown")
        )));
    }
    Ok(body)
}

async fn get_json(state: &AppState, path: &str) -> ApiResult<Value> {
    let key = &stripe(state)?.secret_key;
    let resp = state
        .http
        .get(format!("{API}{path}"))
        .basic_auth(key, Some(""))
        .send()
        .await
        .map_err(ApiError::internal)?;
    resp.json().await.map_err(ApiError::internal)
}

/// The account's Stripe customer, created on first need.
async fn customer_for(state: &AppState, user: &accounts::UserRow) -> ApiResult<String> {
    if let Some(c) = &user.stripe_customer_id {
        return Ok(c.clone());
    }
    let c = post_form(
        state,
        "/customers",
        &[
            ("email", user.email.clone()),
            ("name", user.display_name.clone()),
            ("metadata[user_id]", user.id.to_string()),
        ],
        Some(&format!("customer-{}", user.id)),
    )
    .await?;
    let id = c["id"]
        .as_str()
        .ok_or_else(|| ApiError::internal("stripe: no customer id"))?
        .to_string();
    metering::set_stripe_customer(&state.db, user.id, &id).await?;
    Ok(id)
}

pub async fn checkout(
    State(state): State<AppState>,
    authed: Authed,
    Json(req): Json<dto::CheckoutRequest>,
) -> ApiResult<axum::Json<dto::RedirectUrl>> {
    let cfg = stripe(&state)?;
    let price = match req.interval.as_str() {
        "month" => cfg.price_monthly.clone(),
        "year" => cfg.price_yearly.clone(),
        _ => return Err(ApiError::field("interval", "interval is month or year")),
    };
    if authed.user.is_pro() {
        return Err(ApiError::Conflict(
            "You're already on Pro — manage it from your account page.".into(),
        ));
    }
    let customer = customer_for(&state, &authed.user).await?;
    let s = post_form(
        &state,
        "/checkout/sessions",
        &[
            ("mode", "subscription".into()),
            ("customer", customer),
            ("client_reference_id", authed.user.id.to_string()),
            ("line_items[0][price]", price),
            ("line_items[0][quantity]", "1".into()),
            ("allow_promotion_codes", "true".into()),
            (
                "subscription_data[metadata][user_id]",
                authed.user.id.to_string(),
            ),
            ("success_url", state.public_link("/account?billing=success")),
            (
                "cancel_url",
                state.public_link("/pricing?billing=cancelled"),
            ),
        ],
        None,
    )
    .await?;
    let url = s["url"]
        .as_str()
        .ok_or_else(|| ApiError::internal("stripe: no checkout url"))?;
    Ok(axum::Json(dto::RedirectUrl {
        url: url.to_string(),
    }))
}

pub async fn portal(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<dto::RedirectUrl>> {
    stripe(&state)?;
    let customer = authed.user.stripe_customer_id.clone().ok_or_else(|| {
        ApiError::BadRequest("There is no billing history on this account yet.".into())
    })?;
    let s = post_form(
        &state,
        "/billing_portal/sessions",
        &[
            ("customer", customer),
            ("return_url", state.public_link("/account")),
        ],
        None,
    )
    .await?;
    let url = s["url"]
        .as_str()
        .ok_or_else(|| ApiError::internal("stripe: no portal url"))?;
    Ok(axum::Json(dto::RedirectUrl {
        url: url.to_string(),
    }))
}

/// Cancel every live subscription of a customer — account deletion must not
/// leave a card being charged for an account that no longer exists.
pub async fn cancel_all(state: &AppState, customer: &str) -> ApiResult<()> {
    if state.cfg.stripe.is_none() {
        return Ok(());
    }
    let list = get_json(
        state,
        &format!("/subscriptions?customer={customer}&status=all&limit=100"),
    )
    .await?;
    for sub in list["data"].as_array().into_iter().flatten() {
        let live = matches!(
            sub["status"].as_str(),
            Some("active" | "trialing" | "past_due" | "unpaid")
        );
        if let (true, Some(id)) = (live, sub["id"].as_str()) {
            let key = &stripe(state)?.secret_key;
            state
                .http
                .delete(format!("{API}/subscriptions/{id}"))
                .basic_auth(key, Some(""))
                .send()
                .await
                .map_err(ApiError::internal)?;
        }
    }
    Ok(())
}

// ── webhook ─────────────────────────────────────────────────────────────────

/// Verify `Stripe-Signature: t=…,v1=…[,v1=…]` over `"{t}.{body}"`.
pub fn verify_signature(secret: &str, header: &str, body: &[u8], now: i64) -> bool {
    let mut t = None;
    let mut sigs = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", v)) => t = v.parse::<i64>().ok(),
            Some(("v1", v)) => sigs.push(v.to_string()),
            _ => {}
        }
    }
    let Some(t) = t else { return false };
    if (now - t).abs() > 300 {
        return false;
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(t.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());
    sigs.iter()
        .any(|s| ct_eq(s.as_bytes(), expected.as_bytes()))
}

fn ts(v: &Value) -> Option<OffsetDateTime> {
    v.as_i64()
        .and_then(|s| OffsetDateTime::from_unix_timestamp(s).ok())
}

pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let cfg = stripe(&state)?;
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("missing signature".into()))?;
    if !verify_signature(
        &cfg.webhook_secret,
        sig,
        &body,
        OffsetDateTime::now_utc().unix_timestamp(),
    ) {
        return Err(ApiError::BadRequest("bad signature".into()));
    }
    let event: Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("bad payload".into()))?;
    let id = event["id"].as_str().unwrap_or_default();
    let kind = event["type"].as_str().unwrap_or_default();
    let obj = &event["data"]["object"];

    let mut tx = state.db.begin().await?;
    if !metering::claim_stripe_event(&mut tx, id, kind).await? {
        return Ok(StatusCode::OK); // already applied
    }
    match kind {
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => {
            let customer = obj["customer"].as_str().unwrap_or_default();
            let user = match accounts::user_by_stripe_customer(&mut *tx, customer).await? {
                Some(u) => Some(u.id),
                None => obj["metadata"]["user_id"]
                    .as_str()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok()),
            };
            if let Some(user) = user {
                let item = &obj["items"]["data"][0];
                // Newer API versions report the period on the item.
                let period_end =
                    ts(&obj["current_period_end"]).or_else(|| ts(&item["current_period_end"]));
                metering::upsert_subscription(
                    &mut tx,
                    obj["id"].as_str().unwrap_or_default(),
                    user,
                    obj["status"].as_str().unwrap_or("canceled"),
                    item["price"]["id"].as_str(),
                    period_end,
                    obj["cancel_at_period_end"].as_bool().unwrap_or(false),
                )
                .await?;
                metering::refresh_plan(&mut tx, user).await?;
                tracing::info!(%user, kind, status = obj["status"].as_str().unwrap_or(""), "subscription updated");
            } else {
                tracing::warn!(customer, kind, "subscription event for an unknown customer");
            }
        }
        _ => {}
    }
    tx.commit().await?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(secret: &str, t: i64, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn stripe_signatures_verify_and_expire() {
        let body = br#"{"id":"evt_1"}"#;
        let t = 1_700_000_000;
        let header = format!("t={t},v1={}", sign("whsec_x", t, body));
        assert!(verify_signature("whsec_x", &header, body, t + 10));
        assert!(
            !verify_signature("whsec_x", &header, body, t + 1000),
            "replayed too late"
        );
        assert!(
            !verify_signature("whsec_y", &header, body, t),
            "wrong secret"
        );
        assert!(
            !verify_signature("whsec_x", &header, b"{}", t),
            "tampered body"
        );
        // Stripe sends several v1 signatures while a secret rotates.
        let rotated = format!("t={t},v1=deadbeef,v1={}", sign("whsec_x", t, body));
        assert!(verify_signature("whsec_x", &rotated, body, t));
    }
}
