//! `/api/v1` — the route table. docs/platform/API.md lists the same routes
//! with their request and response shapes.

pub mod admin;
pub mod ai;
pub mod auth;
pub mod billing;
pub mod common;
pub mod content;
pub mod health;
pub mod me;
pub mod oauth;
pub mod profiles;
pub mod runs;

use crate::state::AppState;
use axum::routing::{get, post, put};
use axum::Router;

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/meta", get(health::meta))
        // Public, CDN-cacheable.
        .route("/content/catalog", get(content::catalog))
        .route("/content/guide", get(content::guide))
        .route("/content/problems/{slug}", get(content::public_problem))
        .route("/content/problems/{slug}/trace", get(content::public_trace))
        // Viewer-dependent.
        .route("/problems/{slug}", get(content::problem))
        .route("/problems/{slug}/trace", post(content::trace))
        // Auth.
        .route("/auth/signup", post(auth::signup))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/session", get(auth::session))
        .route("/auth/verify-email", post(auth::verify_email))
        .route("/auth/verify-email/resend", post(auth::resend_verification))
        .route("/auth/password/forgot", post(auth::forgot_password))
        .route("/auth/password/reset", post(auth::reset_password))
        .route("/auth/password/change", post(auth::change_password))
        .route("/auth/sessions", get(auth::list_sessions))
        .route(
            "/auth/sessions/revoke-others",
            post(auth::revoke_other_sessions),
        )
        .route(
            "/auth/sessions/{id}",
            axum::routing::delete(auth::revoke_session),
        )
        .route("/auth/oauth/{provider}/start", get(oauth::start))
        .route("/auth/oauth/{provider}/callback", get(oauth::callback))
        // Account.
        .route("/me", get(me::get).patch(me::update).delete(me::delete))
        .route("/me/export", get(me::export))
        // Profiles and everything scoped to one.
        .route("/profiles", get(profiles::list).post(profiles::create))
        .route(
            "/profiles/{pid}",
            axum::routing::patch(profiles::update).delete(profiles::delete),
        )
        .route(
            "/profiles/{pid}/settings",
            get(profiles::get_settings).put(profiles::put_settings),
        )
        .route("/profiles/{pid}/progress", get(profiles::progress))
        .route(
            "/profiles/{pid}/progress/{slug}/status",
            put(profiles::set_status),
        )
        .route(
            "/profiles/{pid}/progress/{slug}/favourite",
            put(profiles::set_favourite),
        )
        .route("/profiles/{pid}/stats", get(profiles::stats))
        .route("/profiles/{pid}/import", post(profiles::import))
        .route(
            "/profiles/{pid}/playlists",
            get(profiles::playlists).post(profiles::create_playlist),
        )
        .route(
            "/profiles/{pid}/playlists/{id}",
            axum::routing::patch(profiles::rename_playlist).delete(profiles::delete_playlist),
        )
        .route(
            "/profiles/{pid}/playlists/{id}/items/{slug}",
            put(profiles::add_item).delete(profiles::remove_item),
        )
        .route("/profiles/{pid}/drafts/{slug}", get(profiles::drafts))
        .route(
            "/profiles/{pid}/drafts/{slug}/{lang}",
            put(profiles::save_draft).delete(profiles::delete_draft),
        )
        .route("/profiles/{pid}/runs", post(runs::run))
        .route("/profiles/{pid}/submissions", get(profiles::submissions))
        .route(
            "/profiles/{pid}/submissions/{id}",
            get(profiles::submission),
        )
        // AI.
        .route("/ai/status", get(ai::status))
        .route("/ai/chat", post(ai::chat))
        // Billing.
        .route("/billing/checkout", post(billing::checkout))
        .route("/billing/portal", post(billing::portal))
        .route("/billing/webhook", post(billing::webhook))
        // Admin.
        .route("/admin/overview", get(admin::overview))
}
