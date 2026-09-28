//! Users, OAuth identities, sessions, single-use email tokens and the audit
//! trail — everything that decides *who* is making a request.

use crate::security::token_hash;
use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub email_normalized: String,
    pub email_verified_at: Option<OffsetDateTime>,
    pub password_hash: Option<String>,
    pub display_name: String,
    pub role: String,
    pub plan: String,
    pub plan_renews_at: Option<OffsetDateTime>,
    pub stripe_customer_id: Option<String>,
    pub timezone: String,
    pub failed_logins: i32,
    pub locked_until: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub last_login_at: Option<OffsetDateTime>,
}

impl UserRow {
    pub fn is_pro(&self) -> bool {
        self.plan == "pro"
    }
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
    pub fn verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}

const USER_COLS: &str =
    "id, email, email_normalized, email_verified_at, password_hash, display_name, \
     role, plan, plan_renews_at, stripe_customer_id, timezone, failed_logins, locked_until, \
     created_at, updated_at, last_login_at";

pub struct NewUser<'a> {
    pub email: &'a str,
    pub email_normalized: &'a str,
    pub password_hash: Option<&'a str>,
    pub display_name: &'a str,
    pub timezone: &'a str,
    pub verified: bool,
    pub role: &'a str,
}

pub async fn insert_user(conn: &mut PgConnection, u: NewUser<'_>) -> sqlx::Result<UserRow> {
    sqlx::query_as::<_, UserRow>(&format!(
        "INSERT INTO users (id, email, email_normalized, password_hash, display_name, timezone, \
                            email_verified_at, role)
         VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END, $8)
         RETURNING {USER_COLS}"
    ))
    .bind(Uuid::now_v7())
    .bind(u.email)
    .bind(u.email_normalized)
    .bind(u.password_hash)
    .bind(u.display_name)
    .bind(u.timezone)
    .bind(u.verified)
    .bind(u.role)
    .fetch_one(conn)
    .await
}

pub async fn user_by_id<'e>(db: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(&format!("SELECT {USER_COLS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn user_by_email<'e>(
    db: impl PgExecutor<'e>,
    normalized: &str,
) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {USER_COLS} FROM users WHERE email_normalized = $1"
    ))
    .bind(normalized)
    .fetch_optional(db)
    .await
}

pub async fn user_by_stripe_customer<'e>(
    db: impl PgExecutor<'e>,
    customer: &str,
) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {USER_COLS} FROM users WHERE stripe_customer_id = $1"
    ))
    .bind(customer)
    .fetch_optional(db)
    .await
}

pub async fn oauth_providers<'e>(db: impl PgExecutor<'e>, user: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar("SELECT provider FROM oauth_identities WHERE user_id = $1 ORDER BY provider")
        .bind(user)
        .fetch_all(db)
        .await
}

/// A failed password attempt. Ten in a row lock the account for fifteen
/// minutes; the counter restarts after a lock so the next ten are counted anew.
pub async fn record_failed_login(db: &PgPool, user: Uuid) -> sqlx::Result<Option<OffsetDateTime>> {
    sqlx::query_scalar(
        "UPDATE users SET
            failed_logins = CASE WHEN failed_logins + 1 >= 10 THEN 0 ELSE failed_logins + 1 END,
            locked_until  = CASE WHEN failed_logins + 1 >= 10 THEN now() + interval '15 minutes'
                                 ELSE locked_until END
         WHERE id = $1
         RETURNING locked_until",
    )
    .bind(user)
    .fetch_one(db)
    .await
}

pub async fn record_login(db: &PgPool, user: Uuid, promote_admin: bool) -> sqlx::Result<UserRow> {
    sqlx::query_as::<_, UserRow>(&format!(
        "UPDATE users SET failed_logins = 0, locked_until = NULL, last_login_at = now(),
                role = CASE WHEN $2 THEN 'admin' ELSE role END
         WHERE id = $1 RETURNING {USER_COLS}"
    ))
    .bind(user)
    .bind(promote_admin)
    .fetch_one(db)
    .await
}

pub async fn set_password<'e>(db: impl PgExecutor<'e>, user: Uuid, hash: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = $2, failed_logins = 0, locked_until = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(user)
    .bind(hash)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn mark_verified<'e>(db: impl PgExecutor<'e>, user: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET email_verified_at = COALESCE(email_verified_at, now()), updated_at = now()
         WHERE id = $1",
    )
    .bind(user)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn update_me<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    display_name: Option<&str>,
    timezone: Option<&str>,
) -> sqlx::Result<UserRow> {
    sqlx::query_as::<_, UserRow>(&format!(
        "UPDATE users SET display_name = COALESCE($2, display_name),
                          timezone = COALESCE($3, timezone), updated_at = now()
         WHERE id = $1 RETURNING {USER_COLS}"
    ))
    .bind(user)
    .bind(display_name)
    .bind(timezone)
    .fetch_one(db)
    .await
}

pub async fn delete_user<'e>(db: impl PgExecutor<'e>, user: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user)
        .execute(db)
        .await
        .map(|_| ())
}

/// Postgres is the authority on IANA zone names; asking it avoids shipping a
/// second copy of the tz database that could disagree with the one that
/// actually computes local days.
pub async fn is_valid_timezone(db: &PgPool, tz: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
        .bind(tz)
        .fetch_one(db)
        .await
}

// ── OAuth ───────────────────────────────────────────────────────────────────

pub async fn user_by_identity<'e>(
    db: impl PgExecutor<'e>,
    provider: &str,
    subject: &str,
) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as::<_, UserRow>(&format!(
        "SELECT {} FROM users u JOIN oauth_identities o ON o.user_id = u.id
         WHERE o.provider = $1 AND o.provider_user_id = $2",
        USER_COLS
            .split(", ")
            .map(|c| format!("u.{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
    .bind(provider)
    .bind(subject)
    .fetch_optional(db)
    .await
}

pub async fn link_identity<'e>(
    db: impl PgExecutor<'e>,
    provider: &str,
    subject: &str,
    user: Uuid,
    email: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO oauth_identities (provider, provider_user_id, user_id, email)
         VALUES ($1, $2, $3, $4) ON CONFLICT (provider, provider_user_id) DO NOTHING",
    )
    .bind(provider)
    .bind(subject)
    .bind(user)
    .bind(email)
    .execute(db)
    .await
    .map(|_| ())
}

// ── sessions ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct SessionUser {
    pub session_id: Uuid,
    pub session_created_at: OffsetDateTime,
    pub session_seen_at: OffsetDateTime,
    pub session_expires_at: OffsetDateTime,
    #[sqlx(flatten)]
    pub user: UserRow,
}

/// Sessions slide forward on use, but never past this many days from sign-in.
pub const SESSION_ABSOLUTE_DAYS: i64 = 90;

pub async fn create_session<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    token: &str,
    ttl: std::time::Duration,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> sqlx::Result<(Uuid, OffsetDateTime)> {
    let id = Uuid::now_v7();
    let expires = OffsetDateTime::now_utc() + ttl;
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at, user_agent, ip)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user)
    .bind(token_hash(token))
    .bind(expires)
    .bind(user_agent.map(|s| truncate(s, 400)))
    .bind(ip)
    .execute(db)
    .await?;
    Ok((id, expires))
}

pub async fn session_by_token(db: &PgPool, token: &str) -> sqlx::Result<Option<SessionUser>> {
    let cols = USER_COLS
        .split(", ")
        .map(|c| format!("u.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    sqlx::query_as::<_, SessionUser>(&format!(
        "SELECT s.id AS session_id, s.created_at AS session_created_at,
                s.last_seen_at AS session_seen_at, s.expires_at AS session_expires_at, {cols}
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()"
    ))
    .bind(token_hash(token))
    .fetch_optional(db)
    .await
}

/// Slide a session's expiry forward. Called at most every few minutes per
/// session so an active user does not cost a write per request.
pub async fn touch_session(
    db: &PgPool,
    session: Uuid,
    ttl: std::time::Duration,
    ip: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(&format!(
        "UPDATE sessions SET last_seen_at = now(),
                expires_at = LEAST(now() + $2, created_at + interval '{SESSION_ABSOLUTE_DAYS} days'),
                ip = COALESCE($3, ip)
         WHERE id = $1"
    ))
    .bind(session)
    .bind(ttl)
    .bind(ip)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn revoke_session<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    session: Uuid,
) -> sqlx::Result<bool> {
    let r = sqlx::query(
        "UPDATE sessions SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(session)
    .bind(user)
    .execute(db)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Revoke every live session of a user, optionally keeping one.
pub async fn revoke_all_sessions<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    except: Option<Uuid>,
) -> sqlx::Result<u64> {
    let r = sqlx::query(
        "UPDATE sessions SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL AND id IS DISTINCT FROM $2",
    )
    .bind(user)
    .bind(except)
    .execute(db)
    .await?;
    Ok(r.rows_affected())
}

#[derive(sqlx::FromRow)]
pub struct SessionListRow {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

pub async fn list_sessions(db: &PgPool, user: Uuid) -> sqlx::Result<Vec<SessionListRow>> {
    sqlx::query_as(
        "SELECT id, created_at, last_seen_at, expires_at, user_agent, ip FROM sessions
         WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
         ORDER BY last_seen_at DESC LIMIT 50",
    )
    .bind(user)
    .fetch_all(db)
    .await
}

/// Housekeeping: drop sessions and tokens that can never be used again.
pub async fn purge_expired(db: &PgPool) -> sqlx::Result<u64> {
    let a = sqlx::query(
        "DELETE FROM sessions WHERE expires_at < now() - interval '7 days'
            OR revoked_at < now() - interval '7 days'",
    )
    .execute(db)
    .await?
    .rows_affected();
    let b = sqlx::query("DELETE FROM auth_tokens WHERE expires_at < now() - interval '7 days'")
        .execute(db)
        .await?
        .rows_affected();
    Ok(a + b)
}

// ── email tokens ────────────────────────────────────────────────────────────

pub async fn create_auth_token<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    purpose: &str,
    token: &str,
    ttl: std::time::Duration,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO auth_tokens (token_hash, user_id, purpose, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(token_hash(token))
    .bind(user)
    .bind(purpose)
    .bind(OffsetDateTime::now_utc() + ttl)
    .execute(db)
    .await
    .map(|_| ())
}

/// Spend a token: returns its user if it was valid, unused and unexpired, and
/// marks it used in the same statement so two concurrent redemptions cannot
/// both succeed.
pub async fn consume_auth_token(
    conn: &mut PgConnection,
    purpose: &str,
    token: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "UPDATE auth_tokens SET used_at = now()
         WHERE token_hash = $1 AND purpose = $2 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
    )
    .bind(token_hash(token))
    .bind(purpose)
    .fetch_optional(conn)
    .await
}

/// Invalidate outstanding tokens of one purpose (e.g. after a reset, older
/// reset links must stop working).
pub async fn void_auth_tokens<'e>(
    db: impl PgExecutor<'e>,
    user: Uuid,
    purpose: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE auth_tokens SET used_at = now() WHERE user_id = $1 AND purpose = $2 AND used_at IS NULL",
    )
    .bind(user)
    .bind(purpose)
    .execute(db)
    .await
    .map(|_| ())
}

// ── audit ───────────────────────────────────────────────────────────────────

pub async fn audit<'e>(
    db: impl PgExecutor<'e>,
    user: Option<Uuid>,
    kind: &str,
    ip: Option<&str>,
    meta: serde_json::Value,
) {
    let r =
        sqlx::query("INSERT INTO audit_events (user_id, kind, ip, meta) VALUES ($1, $2, $3, $4)")
            .bind(user)
            .bind(kind)
            .bind(ip)
            .bind(meta)
            .execute(db)
            .await;
    // The audit trail must never be the reason a login fails.
    if let Err(e) = r {
        tracing::warn!(error = %e, kind, "audit write failed");
    }
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
