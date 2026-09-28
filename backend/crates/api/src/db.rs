//! The Postgres pool and schema migrations.
//!
//! Migrations are embedded in the binary (`sqlx::migrate!`), so the image that
//! runs the code is the image that knows its schema — there is no separate
//! migrations artifact to keep in step. sqlx takes a Postgres advisory lock
//! while migrating, so the one-off `dsa-api migrate` task and a developer's
//! `RUN_MIGRATIONS=true` can never apply the same migration twice.

use crate::config::Config;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn connect(cfg: &Config) -> anyhow::Result<PgPool> {
    connect_with(cfg.db.clone(), cfg.db_max_connections).await
}

/// Connect, retrying for about a minute. A task routinely starts before its
/// database is reachable — compose brings services up together, an RDS
/// failover takes tens of seconds — and dying on the first refusal turns a
/// blip into a crash loop.
pub async fn connect_with(
    opts: sqlx::postgres::PgConnectOptions,
    max: u32,
) -> anyhow::Result<PgPool> {
    let mut delay = Duration::from_millis(500);
    let mut attempt = 1;
    loop {
        let res = PgPoolOptions::new()
            .max_connections(max)
            .min_connections(1)
            // Fail a request fast rather than queueing it behind a saturated
            // pool; the load balancer's other targets are a better place for it.
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect_with(opts.clone())
            .await;
        match res {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < 12 => {
                tracing::warn!(attempt, error = %e, "database not reachable yet; retrying in {delay:?}");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(8));
                attempt += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}
