//! Usage metering, subscriptions and the admin overview.

use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

/// AI requests used today (UTC day — quotas reset at one global moment, which
/// is what "per day" means on a pricing page).
pub async fn ai_used_today(db: &PgPool, user: Uuid) -> sqlx::Result<i32> {
    Ok(sqlx::query_scalar(
        "SELECT requests FROM ai_usage WHERE user_id = $1 AND day = (now() AT TIME ZONE 'UTC')::date",
    )
    .bind(user)
    .fetch_optional(db)
    .await?
    .unwrap_or(0))
}

/// Reserve one request against the daily quota, atomically. Returns the new
/// count, or `None` when the quota is already spent — a check-then-increment
/// in two statements would let concurrent requests overshoot it.
pub async fn reserve_ai_request(db: &PgPool, user: Uuid, limit: i32) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar(
        "INSERT INTO ai_usage (user_id, day, requests) VALUES ($1, (now() AT TIME ZONE 'UTC')::date, 1)
         ON CONFLICT (user_id, day) DO UPDATE SET requests = ai_usage.requests + 1
         WHERE ai_usage.requests < $2
         RETURNING requests",
    )
    .bind(user)
    .bind(limit)
    .fetch_optional(db)
    .await
}

/// Give a reservation back (the provider failed before producing anything).
pub async fn release_ai_request(db: &PgPool, user: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE ai_usage SET requests = GREATEST(requests - 1, 0)
         WHERE user_id = $1 AND day = (now() AT TIME ZONE 'UTC')::date",
    )
    .bind(user)
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn add_ai_tokens(db: &PgPool, user: Uuid, input: i64, output: i64) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE ai_usage SET input_tokens = input_tokens + $2, output_tokens = output_tokens + $3
         WHERE user_id = $1 AND day = (now() AT TIME ZONE 'UTC')::date",
    )
    .bind(user)
    .bind(input)
    .bind(output)
    .execute(db)
    .await
    .map(|_| ())
}

// ── billing ─────────────────────────────────────────────────────────────────

/// Record a webhook event id; false when it was already processed.
pub async fn claim_stripe_event(
    conn: &mut PgConnection,
    id: &str,
    kind: &str,
) -> sqlx::Result<bool> {
    let r =
        sqlx::query("INSERT INTO stripe_events (id, type) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id)
            .bind(kind)
            .execute(conn)
            .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn set_stripe_customer(db: &PgPool, user: Uuid, customer: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET stripe_customer_id = $2 WHERE id = $1")
        .bind(user)
        .bind(customer)
        .execute(db)
        .await
        .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_subscription(
    conn: &mut PgConnection,
    id: &str,
    user: Uuid,
    status: &str,
    price_id: Option<&str>,
    period_end: Option<OffsetDateTime>,
    cancel_at_period_end: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO subscriptions (id, user_id, status, price_id, current_period_end, cancel_at_period_end)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO UPDATE SET status = excluded.status, price_id = excluded.price_id,
             current_period_end = excluded.current_period_end,
             cancel_at_period_end = excluded.cancel_at_period_end, updated_at = now()",
    )
    .bind(id)
    .bind(user)
    .bind(status)
    .bind(price_id)
    .bind(period_end)
    .bind(cancel_at_period_end)
    .execute(conn)
    .await
    .map(|_| ())
}

/// Re-derive the denormalized `users.plan` from the subscriptions table: Pro
/// while any subscription is active or trialing (Stripe keeps `past_due`
/// subscriptions retrying, and access continues during that grace period).
pub async fn refresh_plan(conn: &mut PgConnection, user: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET
            plan = CASE WHEN EXISTS (
                SELECT 1 FROM subscriptions WHERE user_id = $1
                  AND status IN ('active', 'trialing', 'past_due')) THEN 'pro' ELSE 'free' END,
            plan_renews_at = (SELECT max(current_period_end) FROM subscriptions
                              WHERE user_id = $1 AND status IN ('active', 'trialing', 'past_due')),
            updated_at = now()
         WHERE id = $1",
    )
    .bind(user)
    .execute(conn)
    .await
    .map(|_| ())
}

// ── admin ───────────────────────────────────────────────────────────────────

pub struct Overview {
    pub users: i64,
    pub users_pro: i64,
    pub signups_7d: i64,
    pub runs_24h: i64,
    pub ai_requests_24h: i64,
}

pub async fn overview(db: &PgPool) -> sqlx::Result<Overview> {
    let (users, users_pro, signups_7d): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE plan = 'pro'),
                count(*) FILTER (WHERE created_at > now() - interval '7 days') FROM users",
    )
    .fetch_one(db)
    .await?;
    let runs_24h: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM submissions WHERE created_at > now() - interval '24 hours'",
    )
    .fetch_one(db)
    .await?;
    let ai_requests_24h: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(requests), 0)::bigint FROM ai_usage WHERE day >= (now() AT TIME ZONE 'UTC')::date - 1",
    )
    .fetch_one(db)
    .await?;
    Ok(Overview {
        users,
        users_pro,
        signups_7d,
        runs_24h,
        ai_requests_24h,
    })
}
