//! Profiles and everything scoped to one — the desktop's per-profile store,
//! behind `/profiles/{pid}`. Every handler first proves the profile belongs to
//! the caller (`own_profile`), answering 404 otherwise.

use crate::dto;
use crate::error::{is_unique_violation, ApiError, ApiResult};
use crate::extract::{Authed, Json, Path, Query};
use crate::routes::common::{clean_name, known_slug, own_profile};
use crate::routes::health::{dsa_avatars, dsa_colors};
use crate::state::AppState;
use crate::store::practice;
use axum::extract::State;
use axum::http::StatusCode;
use dsa_core::problem::Tier;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const MAX_PLAYLISTS: i64 = 50;
const MAX_PLAYLIST_ITEMS: i64 = 500;

// ─────────────────────────────────────────────────────────────────────────────
// Profiles
// ─────────────────────────────────────────────────────────────────────────────

fn check_face(avatar: &str, color: &str) -> ApiResult<()> {
    if !dsa_avatars().contains(&avatar) {
        return Err(ApiError::field("avatar", "Pick one of the faces on offer."));
    }
    if !dsa_colors().contains(&color) {
        return Err(ApiError::field(
            "color",
            "Pick one of the colours on offer.",
        ));
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<axum::Json<Vec<dto::Profile>>> {
    Ok(axum::Json(
        practice::profiles(&state.db, authed.user.id).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    authed: Authed,
    Json(req): Json<dto::ProfileInput>,
) -> ApiResult<(StatusCode, axum::Json<dto::Profile>)> {
    let name = clean_name("name", &req.name, 32)?;
    check_face(&req.avatar, &req.color)?;
    let mut tx = state.db.begin().await?;
    practice::lock_user(&mut tx, authed.user.id).await?;
    let max = state.cfg.policy.profiles_per_account as i64;
    if practice::count_profiles(&mut tx, authed.user.id).await? >= max {
        return Err(ApiError::field(
            "name",
            format!("An account can have up to {max} profiles."),
        ));
    }
    let id = match practice::insert_profile(&mut tx, authed.user.id, &name, &req.avatar, &req.color)
        .await
    {
        Ok(id) => id,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::Conflict(format!("“{name}” already exists.")))
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    let p = practice::profile(&state.db, authed.user.id, id)
        .await?
        .ok_or_else(|| ApiError::internal("profile vanished"))?;
    Ok((StatusCode::CREATED, axum::Json(p)))
}

pub async fn update(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Json(req): Json<dto::ProfilePatch>,
) -> ApiResult<axum::Json<dto::Profile>> {
    let name = match req.name.as_deref() {
        Some(n) => Some(clean_name("name", n, 32)?),
        None => None,
    };
    if let Some(a) = &req.avatar {
        check_face(a, dsa_colors()[0])?;
    }
    if let Some(c) = &req.color {
        check_face(dsa_avatars()[0], c)?;
    }
    match practice::update_profile(
        &state.db,
        authed.user.id,
        pid,
        name.as_deref(),
        req.avatar.as_deref(),
        req.color.as_deref(),
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => return Err(ApiError::not_found("profile")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::Conflict(format!(
                "“{}” already exists.",
                name.unwrap_or_default()
            )))
        }
        Err(e) => return Err(e.into()),
    }
    let p = practice::profile(&state.db, authed.user.id, pid)
        .await?
        .ok_or_else(|| ApiError::not_found("profile"))?;
    Ok(axum::Json(p))
}

pub async fn delete(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = state.db.begin().await?;
    practice::lock_user(&mut tx, authed.user.id).await?;
    if practice::count_profiles(&mut tx, authed.user.id).await? <= 1 {
        return Err(ApiError::Conflict(
            "An account needs at least one profile.".into(),
        ));
    }
    if !practice::delete_profile(&mut tx, authed.user.id, pid).await? {
        return Err(ApiError::not_found("profile"));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── settings ────────────────────────────────────────────────────────────────

/// Validate a settings merge-patch against the desktop `Settings` shape, so a
/// client bug (or a hostile one) cannot park megabytes of junk in a profile.
fn validate_settings(patch: &Value) -> ApiResult<Map<String, Value>> {
    let obj = patch
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("settings must be a JSON object".into()))?;
    if serde_json::to_vec(patch).map(|v| v.len()).unwrap_or(0) > 32 * 1024 {
        return Err(ApiError::BadRequest("settings are too large".into()));
    }
    let bad = |k: &str| ApiError::field(k, format!("invalid value for {k}"));
    for (k, v) in obj {
        let ok = match k.as_str() {
            "lang" => v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 16),
            "tier" => v
                .as_str()
                .is_some_and(|s| serde_json::from_value::<Tier>(Value::String(s.into())).is_ok()),
            "speed" => v.as_f64().is_some_and(|f| (0.05..=16.0).contains(&f)),
            "animate" | "show_logs" | "viz_only" | "favourites_only" | "show_question"
            | "ai_open" | "backdrop" => v.is_boolean(),
            "status_filter" => matches!(v.as_str(), Some("all" | "todo" | "attempted" | "solved")),
            "theme" => matches!(v.as_str(), Some("dark" | "light")),
            "playlist" => v.is_null() || v.as_str().is_some_and(|s| Uuid::parse_str(s).is_ok()),
            "watched" => v.as_array().is_some_and(|a| {
                a.len() <= 500 && a.iter().all(|x| x.as_str().is_some_and(|s| s.len() <= 200))
            }),
            _ => return Err(ApiError::field(k, format!("unknown setting {k:?}"))),
        };
        if !ok {
            return Err(bad(k));
        }
    }
    Ok(obj.clone())
}

pub async fn get_settings(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
) -> ApiResult<axum::Json<Value>> {
    own_profile(&state, &authed, pid).await?;
    Ok(axum::Json(practice::settings(&state.db, pid).await?))
}

pub async fn put_settings(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Json(patch): Json<Value>,
) -> ApiResult<axum::Json<Value>> {
    own_profile(&state, &authed, pid).await?;
    let clean = validate_settings(&patch)?;
    Ok(axum::Json(
        practice::merge_settings(&state.db, pid, &Value::Object(clean)).await?,
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress
// ─────────────────────────────────────────────────────────────────────────────

pub async fn progress(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
) -> ApiResult<axum::Json<dto::ProgressSnapshot>> {
    own_profile(&state, &authed, pid).await?;
    let entries = practice::snapshot(&state.db, pid).await?;
    let stats = practice::stats_of(&entries);
    Ok(axum::Json(dto::ProgressSnapshot { entries, stats }))
}

pub async fn set_status(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, slug)): Path<(Uuid, String)>,
    Json(req): Json<dto::SetStatusRequest>,
) -> ApiResult<axum::Json<dto::ProgressEntry>> {
    own_profile(&state, &authed, pid).await?;
    known_slug(&state, &slug)?;
    if !matches!(req.status.as_str(), "todo" | "attempted" | "solved") {
        return Err(ApiError::field(
            "status",
            "status is todo, attempted or solved",
        ));
    }
    let mut tx = state.db.begin().await?;
    let (entry, newly) = practice::set_status(&mut tx, pid, &slug, &req.status).await?;
    if newly {
        practice::bump_activity(&mut tx, pid, &authed.user.timezone, 0, 1).await?;
    }
    tx.commit().await?;
    Ok(axum::Json(entry))
}

pub async fn set_favourite(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, slug)): Path<(Uuid, String)>,
    Json(req): Json<dto::SetFavouriteRequest>,
) -> ApiResult<axum::Json<dto::ProgressEntry>> {
    own_profile(&state, &authed, pid).await?;
    known_slug(&state, &slug)?;
    Ok(axum::Json(
        practice::set_favourite(&state.db, pid, &slug, req.favourite).await?,
    ))
}

/// Merge a desktop export into this profile. Unknown slugs are skipped rather
/// than refused — a desktop build a content release behind may still name a
/// problem that has since been renamed.
pub async fn import(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Json(req): Json<dto::ImportRequest>,
) -> ApiResult<axum::Json<dto::ImportResult>> {
    own_profile(&state, &authed, pid).await?;
    if req.entries.len() > 2000 || req.playlists.len() > MAX_PLAYLISTS as usize {
        return Err(ApiError::BadRequest(
            "that export is larger than any catalogue".into(),
        ));
    }
    let mut tx = state.db.begin().await?;
    let mut rows = 0;
    for (slug, e) in &req.entries {
        if !state.content.has(slug) || !matches!(e.status.as_str(), "todo" | "attempted" | "solved")
        {
            continue;
        }
        practice::import_entry(
            &mut tx,
            pid,
            slug,
            &e.status,
            e.favourite,
            e.attempts.min(1_000_000),
        )
        .await?;
        rows += 1;
    }
    let mut lists = 0;
    for pl in &req.playlists {
        let Ok(name) = clean_name("name", &pl.name, 64) else {
            continue;
        };
        let id = match practice::playlist_by_name(&mut tx, pid, &name).await? {
            Some(id) => id,
            None => {
                if practice::count_playlists(&mut tx, pid).await? >= MAX_PLAYLISTS {
                    continue;
                }
                practice::insert_playlist(&mut tx, pid, &name).await?
            }
        };
        for slug in pl
            .slugs
            .iter()
            .filter(|s| state.content.has(s))
            .take(MAX_PLAYLIST_ITEMS as usize)
        {
            practice::add_item(&mut *tx, id, slug).await?;
        }
        lists += 1;
    }
    tx.commit().await?;
    Ok(axum::Json(dto::ImportResult {
        progress_rows: rows,
        playlists: lists,
    }))
}

// ── stats ───────────────────────────────────────────────────────────────────

/// Current and longest streaks over sorted active days, where "current" may
/// end today or yesterday (a streak is not broken until a whole day passes).
pub fn streaks(days: &[time::Date], today: time::Date) -> (u32, u32, bool) {
    let mut longest = 0u32;
    let mut run = 0u32;
    let mut prev: Option<time::Date> = None;
    for d in days {
        run = match prev {
            Some(p) if p.next_day() == Some(*d) => run + 1,
            Some(p) if p == *d => run,
            _ => 1,
        };
        longest = longest.max(run);
        prev = Some(*d);
    }
    let active_today = days.last() == Some(&today);
    let yesterday = today.previous_day();
    let current = match days.last() {
        Some(last) if *last == today || Some(*last) == yesterday => {
            let mut n = 0u32;
            let mut expect = *last;
            for d in days.iter().rev() {
                if *d == expect {
                    n += 1;
                    match expect.previous_day() {
                        Some(p) => expect = p,
                        None => break,
                    }
                } else if *d < expect {
                    break;
                }
            }
            n
        }
        _ => 0,
    };
    (current, longest, active_today)
}

pub async fn stats(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
) -> ApiResult<axum::Json<dto::Stats>> {
    own_profile(&state, &authed, pid).await?;
    let db = &state.db;
    let entries = practice::snapshot(db, pid).await?;
    let base = practice::stats_of(&entries);
    let status = |slug: &str| {
        entries
            .get(slug)
            .map(|e| e.status.as_str())
            .unwrap_or("todo")
    };
    let catalog = &state.content.lib.catalog;

    let by_tier = Tier::ALL
        .iter()
        .map(|t| {
            let rows: Vec<_> = catalog.iter().filter(|c| t.contains(c.tier)).collect();
            dto::TierStat {
                tier: *t,
                title: t.title(),
                solved: rows.iter().filter(|c| status(&c.slug) == "solved").count(),
                total: rows.len(),
            }
        })
        .collect();

    let mut cats: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    for c in catalog {
        let e = cats.entry(c.category.as_str()).or_default();
        e.2 += 1;
        match status(&c.slug) {
            "solved" => e.0 += 1,
            "attempted" => e.1 += 1,
            _ => {}
        }
    }
    // Keep the catalogue's category order, not alphabetical.
    let by_category = state
        .content
        .lib
        .categories
        .iter()
        .filter_map(|name| {
            cats.get(name.as_str()).map(|(s, a, t)| dto::CategoryStat {
                category: name.clone(),
                solved: *s,
                attempted: *a,
                total: *t,
            })
        })
        .collect();

    use dsa_core::problem::Difficulty;
    let by_difficulty = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard]
        .iter()
        .map(|d| {
            let rows: Vec<_> = catalog.iter().filter(|c| c.difficulty == *d).collect();
            dto::DifficultyStat {
                difficulty: *d,
                solved: rows.iter().filter(|c| status(&c.slug) == "solved").count(),
                total: rows.len(),
            }
        })
        .collect();

    let activity_rows = practice::activity(db, pid).await?;
    let today = practice::local_today(db, &authed.user.timezone).await?;
    let active: Vec<time::Date> = activity_rows
        .iter()
        .filter(|r| r.runs > 0 || r.solved > 0)
        .map(|r| r.day)
        .collect();
    let (current, longest, active_today) = streaks(&active, today);
    let since = today - time::Duration::days(365);
    let activity = activity_rows
        .iter()
        .filter(|r| r.day > since)
        .map(|r| dto::ActivityDay {
            day: r.day.to_string(),
            runs: r.runs,
            solved: r.solved,
        })
        .collect();

    Ok(axum::Json(dto::Stats {
        totals: dto::Totals {
            solved: base.solved,
            attempted: base.attempted,
            favourites: base.favourites,
            submissions: practice::count_submissions(db, pid).await?,
        },
        by_tier,
        by_category,
        by_difficulty,
        streak: dto::Streak {
            current,
            longest,
            active_today,
        },
        activity,
        recent: practice::submissions(db, pid, None, None, 10).await?,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Playlists
// ─────────────────────────────────────────────────────────────────────────────

pub async fn playlists(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
) -> ApiResult<axum::Json<Vec<dto::Playlist>>> {
    own_profile(&state, &authed, pid).await?;
    Ok(axum::Json(practice::playlists(&state.db, pid).await?))
}

pub async fn create_playlist(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Json(req): Json<dto::CreatePlaylistRequest>,
) -> ApiResult<(StatusCode, axum::Json<dto::Playlist>)> {
    own_profile(&state, &authed, pid).await?;
    let name = clean_name("name", &req.name, 64)?;
    let slugs: BTreeSet<&String> = req.slugs.iter().collect();
    if slugs.len() as i64 > MAX_PLAYLIST_ITEMS {
        return Err(ApiError::BadRequest(
            "too many problems for one playlist".into(),
        ));
    }
    for s in &slugs {
        known_slug(&state, s)?;
    }
    let mut tx = state.db.begin().await?;
    if practice::count_playlists(&mut tx, pid).await? >= MAX_PLAYLISTS {
        return Err(ApiError::field(
            "name",
            format!("A profile can have up to {MAX_PLAYLISTS} playlists."),
        ));
    }
    let id = match practice::insert_playlist(&mut tx, pid, &name).await {
        Ok(id) => id,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::Conflict(format!("“{name}” already exists.")))
        }
        Err(e) => return Err(e.into()),
    };
    for s in slugs {
        practice::add_item(&mut *tx, id, s).await?;
    }
    let pl = practice::playlist(&mut *tx, pid, id)
        .await?
        .ok_or_else(|| ApiError::internal("playlist vanished"))?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, axum::Json(pl)))
}

pub async fn rename_playlist(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<dto::RenamePlaylistRequest>,
) -> ApiResult<axum::Json<dto::Playlist>> {
    own_profile(&state, &authed, pid).await?;
    let name = clean_name("name", &req.name, 64)?;
    match practice::rename_playlist(&state.db, pid, id, &name).await {
        Ok(true) => {}
        Ok(false) => return Err(ApiError::not_found("playlist")),
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::Conflict(format!("“{name}” already exists.")))
        }
        Err(e) => return Err(e.into()),
    }
    Ok(axum::Json(
        practice::playlist(&state.db, pid, id)
            .await?
            .ok_or_else(|| ApiError::not_found("playlist"))?,
    ))
}

pub async fn delete_playlist(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    own_profile(&state, &authed, pid).await?;
    if practice::delete_playlist(&state.db, pid, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("playlist"))
    }
}

pub async fn add_item(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, id, slug)): Path<(Uuid, Uuid, String)>,
) -> ApiResult<StatusCode> {
    own_profile(&state, &authed, pid).await?;
    known_slug(&state, &slug)?;
    if !practice::owns_playlist(&state.db, pid, id).await? {
        return Err(ApiError::not_found("playlist"));
    }
    if practice::playlist_len(&state.db, id).await? >= MAX_PLAYLIST_ITEMS {
        return Err(ApiError::BadRequest("this playlist is full".into()));
    }
    practice::add_item(&state.db, id, &slug).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_item(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, id, slug)): Path<(Uuid, Uuid, String)>,
) -> ApiResult<StatusCode> {
    own_profile(&state, &authed, pid).await?;
    if !practice::owns_playlist(&state.db, pid, id).await? {
        return Err(ApiError::not_found("playlist"));
    }
    practice::remove_item(&state.db, id, &slug).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Drafts
// ─────────────────────────────────────────────────────────────────────────────

pub async fn drafts(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, slug)): Path<(Uuid, String)>,
) -> ApiResult<axum::Json<BTreeMap<String, dto::Draft>>> {
    own_profile(&state, &authed, pid).await?;
    Ok(axum::Json(practice::drafts(&state.db, pid, &slug).await?))
}

pub async fn save_draft(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, slug, lang)): Path<(Uuid, String, String)>,
    Json(req): Json<dto::SaveDraftRequest>,
) -> ApiResult<StatusCode> {
    own_profile(&state, &authed, pid).await?;
    known_slug(&state, &slug)?;
    if state.content.language(&lang).is_none() {
        return Err(ApiError::not_found("language"));
    }
    if req.code.len() > state.cfg.policy.draft_bytes {
        return Err(ApiError::field("code", "That draft is too large to save."));
    }
    practice::save_draft(&state.db, pid, &slug, &lang, &req.code).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_draft(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, slug, lang)): Path<(Uuid, String, String)>,
) -> ApiResult<StatusCode> {
    own_profile(&state, &authed, pid).await?;
    practice::delete_draft(&state.db, pid, &slug, &lang).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Submissions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SubmissionsQuery {
    slug: Option<String>,
    limit: Option<i64>,
    cursor: Option<String>,
}

pub async fn submissions(
    State(state): State<AppState>,
    authed: Authed,
    Path(pid): Path<Uuid>,
    Query(q): Query<SubmissionsQuery>,
) -> ApiResult<axum::Json<dto::Page<dto::SubmissionSummary>>> {
    own_profile(&state, &authed, pid).await?;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let before = match q.cursor.as_deref().filter(|c| !c.is_empty()) {
        Some(c) => Some(Uuid::parse_str(c).map_err(|_| ApiError::BadRequest("bad cursor".into()))?),
        None => None,
    };
    // One extra row says whether there is a next page, without a count query.
    let mut items =
        practice::submissions(&state.db, pid, q.slug.as_deref(), before, limit + 1).await?;
    let next_cursor = if items.len() as i64 > limit {
        items.truncate(limit as usize);
        items.last().map(|s| s.id.to_string())
    } else {
        None
    };
    Ok(axum::Json(dto::Page { items, next_cursor }))
}

pub async fn submission(
    State(state): State<AppState>,
    authed: Authed,
    Path((pid, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<axum::Json<Value>> {
    own_profile(&state, &authed, pid).await?;
    let (summary, code, result) = practice::submission(&state.db, pid, id)
        .await?
        .ok_or_else(|| ApiError::not_found("submission"))?;
    let progress = practice::entry(&state.db, pid, &summary.slug).await?;
    // `result` holds the run's output; the full RunResult adds identity and
    // the problem's current progress, like the response to the run itself.
    let mut full = serde_json::to_value(&summary).map_err(ApiError::internal)?;
    if let (Some(obj), Some(out)) = (full.as_object_mut(), result.as_object()) {
        for (k, v) in out {
            obj.insert(k.clone(), v.clone());
        }
        obj.insert(
            "progress".into(),
            serde_json::to_value(&progress).map_err(ApiError::internal)?,
        );
    }
    let mut body = serde_json::to_value(&summary).map_err(ApiError::internal)?;
    body["code"] = Value::String(code);
    body["result"] = full;
    Ok(axum::Json(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn streaks_count_consecutive_local_days() {
        let days = [
            date!(2026 - 09 - 01),
            date!(2026 - 09 - 02),
            date!(2026 - 09 - 03),
            date!(2026 - 09 - 10),
            date!(2026 - 09 - 11),
        ];
        // Active yesterday: the streak is still alive.
        assert_eq!(streaks(&days, date!(2026 - 09 - 12)), (2, 3, false));
        // Active today.
        assert_eq!(streaks(&days, date!(2026 - 09 - 11)), (2, 3, true));
        // A full day missed: broken.
        assert_eq!(streaks(&days, date!(2026 - 09 - 13)), (0, 3, false));
        assert_eq!(streaks(&[], date!(2026 - 09 - 13)), (0, 0, false));
    }

    #[test]
    fn settings_patches_are_validated_against_the_desktop_shape() {
        use serde_json::json;
        assert!(validate_settings(
            &json!({"lang": "go", "tier": "150", "speed": 1.5, "watched": ["two-sum::i"]})
        )
        .is_ok());
        assert!(validate_settings(&json!({"tier": "999"})).is_err());
        assert!(validate_settings(&json!({"speed": 1000})).is_err());
        assert!(validate_settings(&json!({"evil": true})).is_err());
        assert!(validate_settings(&json!({"playlist": null})).is_ok());
        assert!(validate_settings(&json!([1, 2])).is_err());
    }
}
