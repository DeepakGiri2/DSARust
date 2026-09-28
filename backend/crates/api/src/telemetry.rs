//! Logs and metrics.
//!
//! Logs are JSON in production (CloudWatch Logs Insights queries fields, not
//! prose) and human-readable locally. Metrics are Prometheus text on a
//! separate port that no load balancer routes to, so scraping them never
//! competes with — or exposes itself to — public traffic.

use crate::config::{Config, LogFormat};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dsa_api=debug,sqlx=warn,tower_http=info"));
    let registry = tracing_subscriber::registry().with(filter);
    match format {
        LogFormat::Json => registry
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_target(true),
            )
            .init(),
        LogFormat::Pretty => registry.with(fmt::layer().compact()).init(),
    }
}

/// Install the global metrics recorder. Idempotent: tests build many apps.
pub fn init_metrics() -> Option<PrometheusHandle> {
    static HANDLE: std::sync::OnceLock<Option<PrometheusHandle>> = std::sync::OnceLock::new();
    HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .set_buckets(&[
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
                ])
                .ok()?
                .install_recorder()
                .ok()
        })
        .clone()
}

/// Serve `/metrics` on its own listener.
pub async fn serve_metrics(cfg: &Config, handle: PrometheusHandle) -> anyhow::Result<()> {
    let Some(addr) = cfg.metrics_bind else {
        return Ok(());
    };
    let app = axum::Router::new().route(
        "/metrics",
        axum::routing::get(move || {
            let h = handle.clone();
            async move { h.render() }
        }),
    );
    // Metrics are an observer, not a dependency: a busy port costs the
    // scrape, never the service.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(%addr, error = %e, "metrics port unavailable; serving without /metrics");
            return Ok(());
        }
    };
    tracing::info!(%addr, "metrics listening");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "metrics server stopped");
        }
    });
    Ok(())
}
