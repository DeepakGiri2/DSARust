//! `dsa-api` — serve the API, run migrations, or check content.

use anyhow::Context;
use clap::{Parser, Subcommand};
use dsa_api::{app, build_state, config::Config, db, store, telemetry};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "dsa-api", version, about = "DSA Visualized cloud API")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Serve HTTP (the default).
    Serve,
    /// Apply pending database migrations and exit — the one-off deploy task.
    Migrate,
    /// Load and validate the content tree, then exit (CI and image builds).
    CheckContent {
        /// Accept catalogue entries that have no pack yet ("coming soon").
        /// Without it, a missing pack fails the check — it is how a build
        /// that lost files shows up.
        #[arg(long)]
        allow_missing_packs: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // A developer's `.env`, if there is one; the environment always wins.
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Migrate => {
            let (db, log) = dsa_api::config::migration_env().context("configuration")?;
            telemetry::init_logging(log);
            let pool = db::connect_with(db, 2)
                .await
                .context("connecting to Postgres")?;
            db::migrate(&pool).await.context("migrating")?;
            tracing::info!("migrations applied");
            Ok(())
        }
        Cmd::CheckContent { allow_missing_packs } => {
            // A build-time gate: needs the content and nothing else.
            let dir = dsa_api::config::content_dir();
            let store = dsa_api::content::ContentStore::load(&dir, vec![], Default::default())?;
            let missing: Vec<&str> = store
                .lib
                .catalog
                .iter()
                .filter(|c| store.pack(&c.slug).is_none())
                .map(|c| c.slug.as_str())
                .collect();
            if !missing.is_empty() && !allow_missing_packs {
                anyhow::bail!(
                    "{} catalogued problem(s) have no pack: {} — was content/ copied whole?",
                    missing.len(),
                    missing.join(", ")
                );
            }
            println!(
                "content ok: {} problems ({} with packs), version {}",
                store.catalog().total,
                store.catalog().total - missing.len(),
                store.version
            );
            Ok(())
        }
        Cmd::Serve => {
            let cfg = Config::from_env().context("configuration")?;
            telemetry::init_logging(cfg.log_format);
            serve(cfg).await
        }
    }
}

async fn serve(cfg: Config) -> anyhow::Result<()> {
    let metrics = telemetry::init_metrics();
    let pool = db::connect(&cfg).await.context("connecting to Postgres")?;
    if cfg.run_migrations {
        db::migrate(&pool).await.context("migrating")?;
    }
    if let Some(handle) = metrics {
        telemetry::serve_metrics(&cfg, handle).await?;
    }
    let bind = cfg.bind;
    let state = build_state(cfg, pool.clone()).await?;

    // Housekeeping: expired sessions and spent tokens. Every task runs it; the
    // deletes are idempotent, so overlap is harmless.
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(3600));
        loop {
            tick.tick().await;
            match store::accounts::purge_expired(&pool).await {
                Ok(n) if n > 0 => tracing::info!(rows = n, "purged expired sessions and tokens"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "purge failed"),
            }
        }
    });

    let app = app::build(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(addr = %bind, "dsa-api listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    tracing::info!("stopped");
    Ok(())
}

/// SIGTERM (ECS stopping a task) or Ctrl-C. In-flight requests finish; the
/// load balancer has already stopped sending new ones by the time ECS signals.
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    tracing::info!("shutting down");
}
