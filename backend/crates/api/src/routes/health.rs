//! Liveness, readiness and `/meta`.

use crate::dto;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::time::Duration;

/// Liveness: the process is up and serving. Never touches a dependency — a
/// database blip must not make the orchestrator kill healthy tasks.
pub async fn healthz() -> &'static str {
    "ok"
}

/// Readiness: this task can serve real traffic (Postgres answers). The load
/// balancer stops routing to a task that fails it and resumes when it passes.
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let db = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(&state.db),
    )
    .await;
    match db {
        Ok(Ok(_)) => (
            StatusCode::OK,
            Json(json!({ "status": "ready", "content_version": state.content.version })),
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable", "reason": "database" })),
        ),
    }
}

pub async fn meta(State(state): State<AppState>) -> Json<dto::Meta> {
    let cfg = &state.cfg;
    let billing = cfg.stripe.is_some();
    let price = |var: &str, default: f64| -> Option<f64> {
        billing.then(|| {
            std::env::var(var)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        })
    };
    let premium: Vec<&str> = cfg.policy.premium_tiers.iter().map(|t| t.title()).collect();
    let mut pro_features = vec![
        format!("{} AI assist requests a day", cfg.policy.ai_pro_daily),
        format!("{} code runs a minute", cfg.policy.runs_pro_per_min),
        "Every animation, every custom input".to_string(),
    ];
    if !premium.is_empty() {
        pro_features.insert(0, format!("Unlocks {}", premium.join(", ")));
    }
    Json(dto::Meta {
        version: env!("CARGO_PKG_VERSION"),
        content_version: state.content.version.clone(),
        features: dto::Features {
            signup: cfg.policy.signup_enabled,
            email_verification_required: cfg.policy.require_verified_email,
            oauth: dto::OAuthFlags {
                github: cfg.github.is_some(),
                google: cfg.google.is_some(),
            },
            billing,
            ai: dto::AiFlags {
                enabled: state.ai.is_some(),
                provider: state
                    .ai
                    .as_ref()
                    .map(|a| a.provider_name.to_string())
                    .unwrap_or_default(),
                model: state
                    .ai
                    .as_ref()
                    .map(|a| a.model.clone())
                    .unwrap_or_default(),
            },
            runner: state.runner.enabled(),
        },
        plans: vec![
            dto::PlanInfo {
                id: "free",
                name: "Free",
                price_monthly: billing.then_some(0.0),
                price_yearly: billing.then_some(0.0),
                features: vec![
                    "Step-through animations for the NeetCode roadmap".to_string(),
                    "Practice editor with real Go, C++ and Java toolchains".to_string(),
                    format!("{} AI assist requests a day", cfg.policy.ai_free_daily),
                    "Progress, favourites and playlists synced across devices".to_string(),
                ],
            },
            dto::PlanInfo {
                id: "pro",
                name: "Pro",
                price_monthly: price("PRICE_PRO_MONTHLY_USD", 15.0),
                price_yearly: price("PRICE_PRO_YEARLY_USD", 120.0),
                features: pro_features,
            },
        ],
        limits: dto::MetaLimits {
            profiles_per_account: cfg.policy.profiles_per_account,
            draft_bytes: cfg.policy.draft_bytes,
        },
        avatars: dsa_avatars().to_vec(),
        colors: dsa_colors().to_vec(),
    })
}

/// The desktop's profile faces (`dsa_store::AVATARS`), duplicated here rather
/// than depending on the SQLite crate for two constant arrays.
pub fn dsa_avatars() -> &'static [&'static str] {
    &[
        "🎓", "🎯", "⚡", "🔮", "🎧", "🕹", "🔑", "🗝", "📐", "🔍", "⭐", "✨",
    ]
}

/// The desktop's card colours (`dsa_store::COLORS`).
pub fn dsa_colors() -> &'static [&'static str] {
    &[
        "#7c6cff", "#22d3ee", "#34d399", "#fbbf24", "#f87171", "#f472b6",
    ]
}
