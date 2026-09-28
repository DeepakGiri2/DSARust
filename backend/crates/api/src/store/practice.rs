//! Profiles and everything scoped to one: progress, favourites, playlists,
//! drafts, submissions and the daily activity roll-up.
//!
//! The progress semantics are the desktop's (`crates/dsa-store`), moved from
//! SQLite to Postgres unchanged: a row exists only once a problem is touched,
//! `advance` never downgrades a status (a failing re-run of a solved problem
//! is another attempt, not an un-solve), and `set_status` is the deliberate
//! override behind the "mark solved" toggle.

use crate::dto;
use sqlx::{PgConnection, PgExecutor, PgPool};
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Profiles
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ProfileRow {
    id: Uuid,
    name: String,
    avatar: String,
    color: String,
    created_at: OffsetDateTime,
    last_seen_at: OffsetDateTime,
    solved: i64,
    attempted: i64,
    favourites: i64,
}

impl From<ProfileRow> for dto::Profile {
    fn from(r: ProfileRow) -> Self {
        dto::Profile {
            id: r.id,
            name: r.name,
            avatar: r.avatar,
            color: r.color,
            created_at: r.created_at,
            last_seen_at: r.last_seen_at,
            stats: dto::ProfileStats {
                solved: r.solved,
                attempted: r.attempted,
                favourites: r.favourites,
            },
        }
    }
}

const PROFILE_SELECT: &str = "SELECT p.id, p.name, p.avatar, p.color, p.created_at, p.last_seen_at,
        COALESCE(s.solved, 0) AS solved, COALESCE(s.attempted, 0) AS attempted,
        COALESCE(s.favourites, 0) AS favourites
     FROM profiles p
     LEFT JOIN LATERAL (
        SELECT count(*) FILTER (WHERE status = 'solved')    AS solved,
               count(*) FILTER (WHERE status = 'attempted') AS attempted,
               count(*) FILTER (WHERE favourite)            AS favourites
        FROM progress WHERE profile_id = p.id
     ) s ON true";

/// Every profile of an account, most recently used first — the order a
/// picker wants.
pub async fn profiles<'e>(db: impl PgExecutor<'e>, user: Uuid) -> sqlx::Result<Vec<dto::Profile>> {
    let rows: Vec<ProfileRow> = sqlx::query_as(&format!(
        "{PROFILE_SELECT} WHERE p.user_id = $1 ORDER BY p.last_seen_at DESC, p.created_at ASC"
    ))
    .bind(user)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn profile<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<dto::Profile>> {
    let row: Option<ProfileRow> = sqlx::query_as(&format!(
        "{PROFILE_SELECT} WHERE p.user_id = $1 AND p.id = $2"
    ))
    .bind(user)
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(Into::into))
}

/// True when the profile exists and belongs to the account. Every
/// profile-scoped route checks this first and answers 404 otherwise, so ids
/// cannot be probed across accounts.
pub async fn owns_profile(db: &PgPool, user: Uuid, id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM profiles WHERE id = $1 AND user_id = $2)")
        .bind(id)
        .bind(user)
        .fetch_one(db)
        .await
}

pub async fn insert_profile(
    conn: &mut PgConnection,
    user: Uuid,
    name: &str,
    avatar: &str,
    color: &str,
) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO profiles (id, user_id, name, avatar, color) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(user)
    .bind(name)
    .bind(avatar)
    .bind(color)
    .execute(conn)
    .await?;
    Ok(id)
}

pub async fn count_profiles(conn: &mut PgConnection, user: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM profiles WHERE user_id = $1")
        .bind(user)
        .fetch_one(conn)
        .await
}

/// Serialize concurrent profile creation per account, so two simultaneous
/// requests cannot both pass the per-account limit.
pub async fn lock_user(conn: &mut PgConnection, user: Uuid) -> sqlx::Result<()> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user)
        .execute(conn)
        .await
        .map(|_| ())
}

pub async fn update_profile<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    id: Uuid,
    name: Option<&str>,
    avatar: Option<&str>,
    color: Option<&str>,
) -> sqlx::Result<bool> {
    let r = sqlx::query(
        "UPDATE profiles SET name = COALESCE($3, name), avatar = COALESCE($4, avatar),
                color = COALESCE($5, color)
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user)
    .bind(name)
    .bind(avatar)
    .bind(color)
    .execute(db)
    .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn delete_profile(conn: &mut PgConnection, user: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query("DELETE FROM profiles WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user)
        .execute(conn)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn touch_profile(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE profiles SET last_seen_at = now()
         WHERE id = $1 AND last_seen_at < now() - interval '5 minutes'",
    )
    .bind(id)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn settings(db: &PgPool, id: Uuid) -> sqlx::Result<serde_json::Value> {
    sqlx::query_scalar("SELECT settings FROM profiles WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
}

/// Merge-patch the settings object (`||` on jsonb), returning the result.
pub async fn merge_settings(
    db: &PgPool,
    id: Uuid,
    patch: &serde_json::Value,
) -> sqlx::Result<serde_json::Value> {
    sqlx::query_scalar(
        "UPDATE profiles SET settings = settings || $2 WHERE id = $1 RETURNING settings",
    )
    .bind(id)
    .bind(patch)
    .fetch_one(db)
    .await
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ProgressRow {
    slug: String,
    status: String,
    favourite: bool,
    attempts: i32,
    solved_at: Option<OffsetDateTime>,
    updated_at: OffsetDateTime,
}

impl From<ProgressRow> for (String, dto::ProgressEntry) {
    fn from(r: ProgressRow) -> Self {
        (
            r.slug,
            dto::ProgressEntry {
                status: r.status,
                favourite: r.favourite,
                attempts: r.attempts,
                solved_at: r.solved_at,
                updated_at: r.updated_at,
            },
        )
    }
}

const PROGRESS_COLS: &str = "slug, status, favourite, attempts, solved_at, updated_at";

/// Everything one profile has recorded, in a single query.
pub async fn snapshot(
    db: &PgPool,
    profile: Uuid,
) -> sqlx::Result<BTreeMap<String, dto::ProgressEntry>> {
    let rows: Vec<ProgressRow> = sqlx::query_as(&format!(
        "SELECT {PROGRESS_COLS} FROM progress WHERE profile_id = $1"
    ))
    .bind(profile)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn entry<'e>(
    db: impl PgExecutor<'e>,
    profile: Uuid,
    slug: &str,
) -> sqlx::Result<dto::ProgressEntry> {
    let row: Option<ProgressRow> = sqlx::query_as(&format!(
        "SELECT {PROGRESS_COLS} FROM progress WHERE profile_id = $1 AND slug = $2"
    ))
    .bind(profile)
    .bind(slug)
    .fetch_optional(db)
    .await?;
    Ok(row
        .map(|r| <(String, dto::ProgressEntry)>::from(r).1)
        .unwrap_or_else(dto::ProgressEntry::untouched))
}

pub fn stats_of(entries: &BTreeMap<String, dto::ProgressEntry>) -> dto::ProfileStats {
    dto::ProfileStats {
        solved: entries.values().filter(|e| e.status == "solved").count() as i64,
        attempted: entries.values().filter(|e| e.status == "attempted").count() as i64,
        favourites: entries.values().filter(|e| e.favourite).count() as i64,
    }
}

/// Force a status, including back down to `todo`. Returns the entry and
/// whether this call is what made it solved (for the activity roll-up).
pub async fn set_status(
    conn: &mut PgConnection,
    profile: Uuid,
    slug: &str,
    status: &str,
) -> sqlx::Result<(dto::ProgressEntry, bool)> {
    let was_solved: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM progress WHERE profile_id = $1 AND slug = $2 AND status = 'solved')",
    )
    .bind(profile)
    .bind(slug)
    .fetch_one(&mut *conn)
    .await?;
    let row: ProgressRow = sqlx::query_as(&format!(
        "INSERT INTO progress (profile_id, slug, status, solved_at, updated_at)
         VALUES ($1, $2, $3, CASE WHEN $3 = 'solved' THEN now() END, now())
         ON CONFLICT (profile_id, slug) DO UPDATE SET
             status     = excluded.status,
             solved_at  = CASE WHEN excluded.status = 'solved'
                              THEN COALESCE(progress.solved_at, now()) END,
             updated_at = now()
         RETURNING {PROGRESS_COLS}"
    ))
    .bind(profile)
    .bind(slug)
    .bind(status)
    .fetch_one(&mut *conn)
    .await?;
    let newly = status == "solved" && !was_solved;
    Ok((<(String, dto::ProgressEntry)>::from(row).1, newly))
}

pub async fn set_favourite<'e>(
    db: impl PgExecutor<'e>,
    profile: Uuid,
    slug: &str,
    favourite: bool,
) -> sqlx::Result<dto::ProgressEntry> {
    let row: ProgressRow = sqlx::query_as(&format!(
        "INSERT INTO progress (profile_id, slug, status, favourite, updated_at)
         VALUES ($1, $2, 'todo', $3, now())
         ON CONFLICT (profile_id, slug) DO UPDATE SET favourite = excluded.favourite, updated_at = now()
         RETURNING {PROGRESS_COLS}"
    ))
    .bind(profile)
    .bind(slug)
    .bind(favourite)
    .fetch_one(db)
    .await?;
    Ok(<(String, dto::ProgressEntry)>::from(row).1)
}

/// One more Run or test press: at least `attempted`, `solved` when every test
/// passed — and never a downgrade. Returns the entry and whether this attempt
/// is what solved it.
pub async fn record_attempt(
    conn: &mut PgConnection,
    profile: Uuid,
    slug: &str,
    solved: bool,
) -> sqlx::Result<(dto::ProgressEntry, bool)> {
    let before: Option<String> = sqlx::query_scalar(
        "SELECT status FROM progress WHERE profile_id = $1 AND slug = $2 FOR UPDATE",
    )
    .bind(profile)
    .bind(slug)
    .fetch_optional(&mut *conn)
    .await?;
    let target = if solved { "solved" } else { "attempted" };
    let row: ProgressRow = sqlx::query_as(&format!(
        "INSERT INTO progress (profile_id, slug, status, attempts, solved_at, updated_at)
         VALUES ($1, $2, $3, 1, CASE WHEN $3 = 'solved' THEN now() END, now())
         ON CONFLICT (profile_id, slug) DO UPDATE SET
             status = CASE WHEN excluded.status = 'solved'
                             OR (excluded.status = 'attempted' AND progress.status = 'todo')
                           THEN excluded.status ELSE progress.status END,
             attempts   = progress.attempts + 1,
             solved_at  = CASE WHEN excluded.status = 'solved'
                              THEN COALESCE(progress.solved_at, now()) ELSE progress.solved_at END,
             updated_at = now()
         RETURNING {PROGRESS_COLS}"
    ))
    .bind(profile)
    .bind(slug)
    .bind(target)
    .fetch_one(&mut *conn)
    .await?;
    let newly = solved && before.as_deref() != Some("solved");
    Ok((<(String, dto::ProgressEntry)>::from(row).1, newly))
}

/// Merge a desktop export: statuses only move up, favourites only turn on,
/// attempts take the larger count. Importing twice changes nothing.
pub async fn import_entry(
    conn: &mut PgConnection,
    profile: Uuid,
    slug: &str,
    status: &str,
    favourite: bool,
    attempts: i32,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO progress (profile_id, slug, status, favourite, attempts, solved_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, CASE WHEN $3 = 'solved' THEN now() END, now())
         ON CONFLICT (profile_id, slug) DO UPDATE SET
             status = CASE
                 WHEN progress.status = 'solved' OR excluded.status = 'solved' THEN 'solved'
                 WHEN progress.status = 'attempted' OR excluded.status = 'attempted' THEN 'attempted'
                 ELSE 'todo' END,
             favourite  = progress.favourite OR excluded.favourite,
             attempts   = GREATEST(progress.attempts, excluded.attempts),
             solved_at  = COALESCE(progress.solved_at, excluded.solved_at),
             updated_at = now()",
    )
    .bind(profile)
    .bind(slug)
    .bind(status)
    .bind(favourite)
    .bind(attempts.max(0))
    .execute(conn)
    .await
    .map(|_| ())
}

// ─────────────────────────────────────────────────────────────────────────────
// Activity roll-up
// ─────────────────────────────────────────────────────────────────────────────

/// Count a run (and maybe a solve) against the profile's *local* day. The
/// account's IANA zone decides where midnight is, so a streak kept by someone
/// in Tokyo does not break at 09:00.
pub async fn bump_activity(
    conn: &mut PgConnection,
    profile: Uuid,
    tz: &str,
    runs: i32,
    solved: i32,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO activity_days (profile_id, day, runs, solved)
         VALUES ($1, (now() AT TIME ZONE $2)::date, $3, $4)
         ON CONFLICT (profile_id, day) DO UPDATE SET
             runs = activity_days.runs + excluded.runs,
             solved = activity_days.solved + excluded.solved",
    )
    .bind(profile)
    .bind(tz)
    .bind(runs)
    .bind(solved)
    .execute(conn)
    .await
    .map(|_| ())
}

#[derive(sqlx::FromRow)]
pub struct ActivityRow {
    pub day: time::Date,
    pub runs: i32,
    pub solved: i32,
}

pub async fn activity(db: &PgPool, profile: Uuid) -> sqlx::Result<Vec<ActivityRow>> {
    sqlx::query_as("SELECT day, runs, solved FROM activity_days WHERE profile_id = $1 ORDER BY day")
        .bind(profile)
        .fetch_all(db)
        .await
}

pub async fn local_today(db: &PgPool, tz: &str) -> sqlx::Result<time::Date> {
    sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date")
        .bind(tz)
        .fetch_one(db)
        .await
}

// ─────────────────────────────────────────────────────────────────────────────
// Playlists
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct PlaylistRow {
    id: Uuid,
    name: String,
    created_at: OffsetDateTime,
    slugs: Vec<String>,
}

impl From<PlaylistRow> for dto::Playlist {
    fn from(r: PlaylistRow) -> Self {
        dto::Playlist {
            id: r.id,
            name: r.name,
            slugs: r.slugs,
            created_at: r.created_at,
        }
    }
}

const PLAYLIST_SELECT: &str = "SELECT p.id, p.name, p.created_at,
        COALESCE(array_agg(i.slug ORDER BY i.added_at) FILTER (WHERE i.slug IS NOT NULL), '{}') AS slugs
     FROM playlists p LEFT JOIN playlist_items i ON i.playlist_id = p.id";

pub async fn playlists(db: &PgPool, profile: Uuid) -> sqlx::Result<Vec<dto::Playlist>> {
    let rows: Vec<PlaylistRow> = sqlx::query_as(&format!(
        "{PLAYLIST_SELECT} WHERE p.profile_id = $1 GROUP BY p.id ORDER BY lower(p.name)"
    ))
    .bind(profile)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn playlist<'e>(
    db: impl PgExecutor<'e>,
    profile: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<dto::Playlist>> {
    let row: Option<PlaylistRow> = sqlx::query_as(&format!(
        "{PLAYLIST_SELECT} WHERE p.profile_id = $1 AND p.id = $2 GROUP BY p.id"
    ))
    .bind(profile)
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn count_playlists(conn: &mut PgConnection, profile: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM playlists WHERE profile_id = $1")
        .bind(profile)
        .fetch_one(conn)
        .await
}

pub async fn insert_playlist(
    conn: &mut PgConnection,
    profile: Uuid,
    name: &str,
) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO playlists (id, profile_id, name) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(profile)
        .bind(name)
        .execute(conn)
        .await?;
    Ok(id)
}

pub async fn rename_playlist<'e>(
    db: impl PgExecutor<'e>,
    profile: Uuid,
    id: Uuid,
    name: &str,
) -> sqlx::Result<bool> {
    let r = sqlx::query("UPDATE playlists SET name = $3 WHERE id = $1 AND profile_id = $2")
        .bind(id)
        .bind(profile)
        .bind(name)
        .execute(db)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn delete_playlist(db: &PgPool, profile: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query("DELETE FROM playlists WHERE id = $1 AND profile_id = $2")
        .bind(id)
        .bind(profile)
        .execute(db)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn owns_playlist<'e>(
    db: impl PgExecutor<'e>,
    profile: Uuid,
    id: Uuid,
) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM playlists WHERE id = $1 AND profile_id = $2)")
        .bind(id)
        .bind(profile)
        .fetch_one(db)
        .await
}

pub async fn playlist_len<'e>(db: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM playlist_items WHERE playlist_id = $1")
        .bind(id)
        .fetch_one(db)
        .await
}

/// Adding twice is not an error — the control is a toggle, and a second click
/// from a stale view should not fail.
pub async fn add_item<'e>(db: impl PgExecutor<'e>, id: Uuid, slug: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO playlist_items (playlist_id, slug) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(slug)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn remove_item(db: &PgPool, id: Uuid, slug: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM playlist_items WHERE playlist_id = $1 AND slug = $2")
        .bind(id)
        .bind(slug)
        .execute(db)
        .await
        .map(|_| ())
}

pub async fn playlist_by_name(
    conn: &mut PgConnection,
    profile: Uuid,
    name: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM playlists WHERE profile_id = $1 AND lower(name) = lower($2)")
        .bind(profile)
        .bind(name)
        .fetch_optional(conn)
        .await
}

// ─────────────────────────────────────────────────────────────────────────────
// Drafts
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct DraftRow {
    lang: String,
    code: String,
    updated_at: OffsetDateTime,
}

pub async fn drafts(
    db: &PgPool,
    profile: Uuid,
    slug: &str,
) -> sqlx::Result<BTreeMap<String, dto::Draft>> {
    let rows: Vec<DraftRow> = sqlx::query_as(
        "SELECT lang, code, updated_at FROM drafts WHERE profile_id = $1 AND slug = $2",
    )
    .bind(profile)
    .bind(slug)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.lang.clone(),
                dto::Draft {
                    lang: r.lang,
                    code: r.code,
                    updated_at: r.updated_at,
                },
            )
        })
        .collect())
}

pub async fn save_draft(
    db: &PgPool,
    profile: Uuid,
    slug: &str,
    lang: &str,
    code: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO drafts (profile_id, slug, lang, code, updated_at) VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (profile_id, slug, lang) DO UPDATE SET code = excluded.code, updated_at = now()",
    )
    .bind(profile)
    .bind(slug)
    .bind(lang)
    .bind(code)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn delete_draft(db: &PgPool, profile: Uuid, slug: &str, lang: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM drafts WHERE profile_id = $1 AND slug = $2 AND lang = $3")
        .bind(profile)
        .bind(slug)
        .bind(lang)
        .execute(db)
        .await
        .map(|_| ())
}

// ─────────────────────────────────────────────────────────────────────────────
// Submissions
// ─────────────────────────────────────────────────────────────────────────────

pub struct NewSubmission<'a> {
    pub id: Uuid,
    pub profile: Uuid,
    pub user: Uuid,
    pub slug: &'a str,
    pub lang: &'a str,
    pub kind: &'a str,
    pub mode: &'a str,
    pub code: &'a str,
    pub status: &'a str,
    pub passed: i32,
    pub total: i32,
    pub duration_ms: i32,
    pub result: serde_json::Value,
}

pub async fn insert_submission(
    conn: &mut PgConnection,
    s: NewSubmission<'_>,
) -> sqlx::Result<OffsetDateTime> {
    sqlx::query_scalar(
        "INSERT INTO submissions (id, profile_id, user_id, slug, lang, kind, mode, code, status,
                                  passed, total, duration_ms, result)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING created_at",
    )
    .bind(s.id)
    .bind(s.profile)
    .bind(s.user)
    .bind(s.slug)
    .bind(s.lang)
    .bind(s.kind)
    .bind(s.mode)
    .bind(s.code)
    .bind(s.status)
    .bind(s.passed)
    .bind(s.total)
    .bind(s.duration_ms)
    .bind(s.result)
    .fetch_one(conn)
    .await
}

#[derive(sqlx::FromRow)]
struct SubmissionRow {
    id: Uuid,
    slug: String,
    lang: String,
    kind: String,
    mode: String,
    status: String,
    passed: i32,
    total: i32,
    duration_ms: i32,
    created_at: OffsetDateTime,
}

impl From<SubmissionRow> for dto::SubmissionSummary {
    fn from(r: SubmissionRow) -> Self {
        dto::SubmissionSummary {
            id: r.id,
            slug: r.slug,
            lang: r.lang,
            kind: r.kind,
            mode: r.mode,
            status: r.status,
            passed: r.passed,
            total: r.total,
            duration_ms: r.duration_ms,
            created_at: r.created_at,
        }
    }
}

const SUBMISSION_COLS: &str =
    "id, slug, lang, kind, mode, status, passed, total, duration_ms, created_at";

/// Newest first, keyset-paginated on the time-ordered id (UUIDv7 sorts by
/// creation time), so a deep page costs the same as the first.
pub async fn submissions(
    db: &PgPool,
    profile: Uuid,
    slug: Option<&str>,
    before: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<dto::SubmissionSummary>> {
    let rows: Vec<SubmissionRow> = sqlx::query_as(&format!(
        "SELECT {SUBMISSION_COLS} FROM submissions
         WHERE profile_id = $1 AND ($2::text IS NULL OR slug = $2) AND ($3::uuid IS NULL OR id < $3)
         ORDER BY id DESC LIMIT $4"
    ))
    .bind(profile)
    .bind(slug)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn submission(
    db: &PgPool,
    profile: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<(dto::SubmissionSummary, String, serde_json::Value)>> {
    #[derive(sqlx::FromRow)]
    struct Full {
        #[sqlx(flatten)]
        summary: SubmissionRow,
        code: String,
        result: serde_json::Value,
    }
    let row: Option<Full> = sqlx::query_as(&format!(
        "SELECT {SUBMISSION_COLS}, code, result FROM submissions WHERE profile_id = $1 AND id = $2"
    ))
    .bind(profile)
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| (r.summary.into(), r.code, r.result)))
}

pub async fn count_submissions(db: &PgPool, profile: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM submissions WHERE profile_id = $1")
        .bind(profile)
        .fetch_one(db)
        .await
}
