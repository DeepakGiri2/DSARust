//! Operator overview, for accounts with the `admin` role.

use crate::dto;
use crate::error::{ApiError, ApiResult};
use crate::extract::Authed;
use crate::state::AppState;
use crate::store::metering;
use axum::extract::State;

pub async fn overview(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<dto::AdminOverview>> {
    if !authed.user.is_admin() {
        // Not 403: the page's existence is not something to confirm.
        return Err(ApiError::not_found("page"));
    }
    let o = metering::overview(&state.db).await?;
    Ok(axum::Json(dto::AdminOverview {
        users: o.users,
        users_pro: o.users_pro,
        signups_7d: o.signups_7d,
        runs_24h: o.runs_24h,
        ai_requests_24h: o.ai_requests_24h,
        // A server with broken content refuses to start, so a running one has
        // none; pack warnings are what remain worth surfacing.
        content_errors: state
            .content
            .lib
            .packs()
            .flat_map(|(slug, p)| p.warnings.iter().map(move |w| format!("{slug}: {w}")))
            .collect(),
    }))
}
