//! `dsa-runner`: compiles and runs untrusted Go, C++, Java and Python.
//!
//! Every byte of a submitted program is treated as hostile. The runner holds
//! no secrets worth stealing, but it *is* a machine that runs strangers' code
//! for the next stranger, so the design goal is that one job can neither see
//! nor influence any other job, the runner itself, or the network.
//!
//! The layers, outermost first (see the crate README for the full threat
//! model):
//!
//! 1. **Deployment** — an unprivileged container (compose/ECS) or a Lambda
//!    microVM with no network route at all.
//! 2. **Runner hardening** ([`hardening`]) — non-dumpable, secrets scrubbed
//!    from its own environment, child subreaper so escaped grandchildren come
//!    back to it to be killed.
//! 3. **Per-job isolation** ([`engine`]) — a fresh 0700 directory, a distinct
//!    unprivileged uid when the runner is root, a scrubbed environment, and a
//!    teardown that kills every leftover process and file.
//! 4. **Per-process confinement** (`exec::linux`) — own session, rlimits,
//!    `no_new_privs`, a seccomp filter, wall-clock/RSS/output watchdogs.
//!
//! The same [`engine::Engine`] serves both transports: [`http`] (compose,
//! ECS) and `lambda` (AWS, Linux only). Everything platform-specific is behind
//! `cfg(target_os = "linux")`; elsewhere the crate still builds and its logic
//! is unit-tested, but it refuses jobs unless `RUNNER_ALLOW_UNSANDBOXED=1`.

pub mod classify;
pub mod config;
pub mod engine;
pub mod exec;
pub mod hardening;
pub mod http;
#[cfg(target_os = "linux")]
pub mod lambda;
pub mod output;
pub mod registry;
pub mod slots;

/// The runner's own version, reported in every response and in `/healthz`.
pub const RUNNER_VERSION: &str = env!("CARGO_PKG_VERSION");
