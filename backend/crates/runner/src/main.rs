//! `dsa-runner` entry point: read the configuration, harden the process, pick
//! the transport.
//!
//! Startup is deliberately synchronous up to the point where the async
//! runtime is built: scrubbing secrets from the environment is only sound
//! while the process has a single thread, and the runtime's worker threads do
//! not exist yet.

use dsa_runner::config::Config;
use dsa_runner::engine::Engine;
use dsa_runner::hardening;
use std::process::ExitCode;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("dsa-runner: invalid configuration: {e:#}");
            return ExitCode::FAILURE;
        }
    };
    init_tracing(config.log_json);

    let lambda = std::env::var_os("AWS_LAMBDA_RUNTIME_API").is_some();
    hardening::scrub_secrets();
    if let Err(e) = hardening::harden_process() {
        tracing::error!(error = %e, "could not harden the runner process");
        return ExitCode::FAILURE;
    }

    let engine = match Engine::new(config, lambda) {
        Ok(engine) => Arc::new(engine),
        Err(e) => {
            tracing::error!(error = format!("{e:#}"), "runner failed to start");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(error = %e, "could not start the async runtime");
            return ExitCode::FAILURE;
        }
    };
    let result = runtime.block_on(serve(engine, lambda));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(error = format!("{e:#}"), "runner stopped");
            ExitCode::FAILURE
        }
    }
}

async fn serve(engine: Arc<Engine>, lambda: bool) -> anyhow::Result<()> {
    if lambda {
        #[cfg(target_os = "linux")]
        return dsa_runner::lambda::serve(engine).await;
        #[cfg(not(target_os = "linux"))]
        anyhow::bail!("AWS_LAMBDA_RUNTIME_API is set, but Lambda mode needs Linux");
    }
    dsa_runner::http::serve(engine).await
}

fn init_tracing(json: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if json {
        builder.json().with_current_span(false).init();
    } else {
        builder.init();
    }
}
