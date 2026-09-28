//! DSA Visualized cloud API.
//!
//! A stateless axum service over PostgreSQL: accounts and sessions, the
//! desktop's per-profile progress model, content and traces from the desktop's
//! own engine, sandboxed code execution through `dsa-runner`, AI assist and
//! billing. Every task is interchangeable — state lives in Postgres and Redis —
//! so the service scales by adding tasks behind the load balancer.

pub mod ai;
pub mod app;
pub mod config;
pub mod content;
pub mod db;
pub mod dto;
pub mod error;
pub mod extract;
pub mod mail;
pub mod ratelimit;
pub mod routes;
pub mod runner;
pub mod security;
pub mod state;
pub mod store;
pub mod telemetry;
pub mod traces;

use anyhow::Context;
use config::Config;
use std::sync::Arc;
use std::time::Duration;

/// Build the whole application state from configuration: connect, load
/// content, wire every integration. Shared by `main` and the integration tests.
pub async fn build_state(cfg: Config, db: sqlx::PgPool) -> anyhow::Result<state::AppState> {
    build_state_with(cfg, db, None).await
}

/// `build_state`, with the mailer replaced — the integration tests capture
/// the links that verification and reset emails would carry.
pub async fn build_state_with(
    cfg: Config,
    db: sqlx::PgPool,
    mailer: Option<Box<dyn mail::Mailer>>,
) -> anyhow::Result<state::AppState> {
    let http = reqwest::Client::builder()
        .user_agent(concat!("dsa-api/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?;

    // Languages come from content; which of them can run depends on the runner.
    let languages = dsa_content::Library::load(&cfg.content_dir).languages;
    let runner = runner::Runner::from_config(&cfg, http.clone(), &languages).await?;
    let content = Arc::new(
        content::ContentStore::load(
            &cfg.content_dir,
            cfg.policy.premium_tiers.clone(),
            runner.languages().clone(),
        )
        .with_context(|| format!("loading content from {}", cfg.content_dir.display()))?,
    );
    tracing::info!(
        problems = content.catalog().total,
        version = %content.version,
        runner = runner.label,
        "content loaded"
    );

    let (limiter, redis): (Box<dyn ratelimit::RateLimiter>, _) = match &cfg.redis_url {
        Some(url) => {
            let client = redis::Client::open(url.as_str()).context("REDIS_URL")?;
            let conn = redis::aio::ConnectionManager::new(client)
                .await
                .context("connecting to Redis")?;
            (
                Box::new(ratelimit::RedisLimiter::new(conn.clone())),
                Some(conn),
            )
        }
        None => {
            if cfg.is_production() {
                tracing::warn!("no REDIS_URL: rate limits are per task, not fleet-wide");
            }
            (Box::new(ratelimit::MemoryLimiter::default()), None)
        }
    };

    let mailer = match mailer {
        Some(m) => m,
        None => mail::from_config(&cfg).await?,
    };
    let ai = ai::Assistant::from_config(&cfg, http.clone(), &cfg.content_dir).await?;
    let traces = traces::TraceService::new(content.clone());

    Ok(state::AppState::new(state::Inner {
        cfg,
        db,
        content,
        traces,
        runner,
        mailer,
        limiter,
        ai,
        http,
        redis,
    }))
}
