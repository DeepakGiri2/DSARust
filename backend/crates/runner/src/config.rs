//! Runner configuration, read once from the environment at startup.
//!
//! The variables are the ones `docs/platform/SERVICES.md` defines for
//! `dsa-runner`, plus a few ceilings that page does not need to mention
//! because their defaults are right for every deployment. Every value a
//! request can influence is a *ceiling* here: [`Ceilings::clamp`] is the only
//! way a request's [`Limits`] become the [`EffectiveLimits`] a job runs under,
//! so there is exactly one place where "never trust the caller" is enforced.

use anyhow::{bail, Context, Result};
use dsa_protocol::{Limits, MAX_CASES, MAX_SOURCE_BYTES, MAX_STDIN_BYTES};
use std::path::PathBuf;
use std::time::Duration;

/// Everything the runner reads from its environment.
#[derive(Clone, Debug)]
pub struct Config {
    /// `http` mode listen address.
    pub bind: String,
    /// Bearer token `http` mode requires, when set.
    pub token: Option<String>,
    /// Simultaneous jobs; excess requests are refused with `busy`.
    pub max_concurrency: usize,
    /// Scratch root; one subdirectory per job, removed afterwards.
    pub work_dir: PathBuf,
    /// Read-only Go build cache baked into the image (see the README).
    pub go_warm_cache: PathBuf,
    pub ceilings: Ceilings,
    /// Largest request body `http` mode accepts.
    pub max_body_bytes: usize,
    /// Run jobs without the Linux sandbox (non-Linux development only).
    pub allow_unsandboxed: bool,
    /// Emit JSON log lines instead of human-readable ones.
    pub log_json: bool,
}

/// The most any request may ask for. Requests asking for more get this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceilings {
    pub run_timeout: Duration,
    pub compile_timeout: Duration,
    pub total_timeout: Duration,
    pub memory_mb: u64,
    /// Compilers need far more than the programs they build: `g++ -O2` on a
    /// file including `<bits/stdc++.h>` peaks at several hundred MiB, and
    /// `javac` is itself a JVM. This is not requestable — every compile gets
    /// it — so a request cannot starve its own compiler into a confusing
    /// failure, and cannot inflate it either.
    pub compile_memory_mb: u64,
    pub max_output_bytes: u64,
}

/// Floors keep a request from asking for limits so small that the result is
/// meaningless noise (a 1 ms timeout reports every program as too slow).
const MIN_RUN_TIMEOUT: Duration = Duration::from_millis(100);
const MIN_COMPILE_TIMEOUT: Duration = Duration::from_millis(500);
const MIN_TOTAL_TIMEOUT: Duration = Duration::from_millis(1000);
const MIN_MEMORY_MB: u64 = 32;
const MIN_OUTPUT_BYTES: u64 = 256;

/// The limits one job actually runs under, after clamping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectiveLimits {
    pub run_timeout: Duration,
    pub compile_timeout: Duration,
    pub total_timeout: Duration,
    pub memory_mb: u64,
    pub compile_memory_mb: u64,
    pub max_output_bytes: usize,
}

impl Ceilings {
    /// Clamp every requested limit into `[floor, ceiling]`.
    ///
    /// Zero, absurdly large and missing values all end up somewhere sane; no
    /// value the caller sends can raise anything above what the operator set.
    pub fn clamp(&self, req: &Limits) -> EffectiveLimits {
        let ms = |v: u64, lo: Duration, hi: Duration| {
            let hi = hi.max(lo);
            Duration::from_millis(v).clamp(lo, hi)
        };
        let memory_mb = req
            .memory_mb
            .clamp(MIN_MEMORY_MB, self.memory_mb.max(MIN_MEMORY_MB));
        let max_output = req.max_output_bytes.clamp(
            MIN_OUTPUT_BYTES,
            self.max_output_bytes.max(MIN_OUTPUT_BYTES),
        );
        EffectiveLimits {
            run_timeout: ms(req.run_timeout_ms, MIN_RUN_TIMEOUT, self.run_timeout),
            compile_timeout: ms(
                req.compile_timeout_ms,
                MIN_COMPILE_TIMEOUT,
                self.compile_timeout,
            ),
            total_timeout: ms(req.total_timeout_ms, MIN_TOTAL_TIMEOUT, self.total_timeout),
            memory_mb,
            // Never below what the program itself may use: a compiler that
            // gets less than its output would be a strange inversion.
            compile_memory_mb: self.compile_memory_mb.max(memory_mb),
            max_output_bytes: usize::try_from(max_output).unwrap_or(usize::MAX),
        }
    }
}

/// The largest body a valid [`dsa_protocol::ExecuteRequest`] can have, plus
/// room for JSON escaping and field names.
///
/// The protocol allows 64 cases of 256 KiB stdin each, so a fixed 1 MiB cap
/// would reject requests `dsa-protocol` itself calls valid. The structural
/// limits in `ExecuteRequest::validate` are the real bound; this only stops a
/// body far beyond them from being buffered at all. Worst-case JSON escaping
/// (`\u00XX`, six bytes per input byte) is not budgeted: realistic stdin is
/// digits and spaces, and a pathological body is rejected cleanly with 413.
pub fn default_max_body_bytes() -> usize {
    MAX_SOURCE_BYTES + MAX_CASES * MAX_STDIN_BYTES + 1024 * 1024
}

impl Config {
    /// Read the process environment.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Build from any key lookup, so tests do not have to mutate the real
    /// (process-global) environment.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |k: &str| {
            get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let num = |k: &str, default: u64| -> Result<u64> {
            match get(k) {
                None => Ok(default),
                Some(v) => v
                    .parse::<u64>()
                    .with_context(|| format!("{k} must be a non-negative integer, got {v:?}")),
            }
        };

        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
        let max_concurrency = usize::try_from(num("RUNNER_MAX_CONCURRENCY", cpus as u64)?)
            .context("RUNNER_MAX_CONCURRENCY is too large")?;
        if max_concurrency == 0 {
            bail!("RUNNER_MAX_CONCURRENCY must be at least 1");
        }
        // Job uids are 20000 + slot; keep them well inside the 16-bit-safe range
        // (see `slots::UID_BASE`).
        if max_concurrency > crate::slots::MAX_SLOTS {
            bail!(
                "RUNNER_MAX_CONCURRENCY must be at most {}",
                crate::slots::MAX_SLOTS
            );
        }

        let ceilings = Ceilings {
            run_timeout: Duration::from_millis(num("RUNNER_MAX_RUN_TIMEOUT_MS", 10_000)?),
            compile_timeout: Duration::from_millis(num("RUNNER_MAX_COMPILE_TIMEOUT_MS", 30_000)?),
            total_timeout: Duration::from_millis(num("RUNNER_MAX_TOTAL_TIMEOUT_MS", 60_000)?),
            memory_mb: num("RUNNER_MAX_MEMORY_MB", 512)?,
            compile_memory_mb: num("RUNNER_MAX_COMPILE_MEMORY_MB", 2048)?,
            max_output_bytes: num("RUNNER_MAX_OUTPUT_BYTES", 1024 * 1024)?,
        };

        let max_body_bytes = match get("RUNNER_MAX_BODY_BYTES") {
            None => default_max_body_bytes(),
            Some(v) => v
                .parse::<usize>()
                .with_context(|| format!("RUNNER_MAX_BODY_BYTES must be an integer, got {v:?}"))?,
        };

        let flag = |k: &str| {
            get(k).is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        };

        let work_dir =
            get("RUNNER_WORK_DIR").map_or_else(|| PathBuf::from("/tmp/dsa-runner"), PathBuf::from);
        // The runner chmods its work root to 0711 and sweeps the shared temp
        // directories itself; pointing it *at* one of them would break every
        // other user of that directory.
        if ["/", "/tmp", "/var/tmp", "/dev/shm"]
            .iter()
            .any(|shared| work_dir == std::path::Path::new(shared))
        {
            bail!(
                "RUNNER_WORK_DIR must be a dedicated directory, not {}",
                work_dir.display()
            );
        }
        if cfg!(unix) && !work_dir.is_absolute() {
            bail!("RUNNER_WORK_DIR must be an absolute path");
        }

        Ok(Self {
            bind: get("RUNNER_BIND").unwrap_or_else(|| "0.0.0.0:8081".into()),
            token: get("RUNNER_TOKEN"),
            max_concurrency,
            work_dir,
            go_warm_cache: get("RUNNER_GO_WARM_CACHE")
                .map_or_else(|| PathBuf::from("/opt/dsa-runner/go-cache"), PathBuf::from),
            ceilings,
            max_body_bytes,
            allow_unsandboxed: flag("RUNNER_ALLOW_UNSANDBOXED"),
            log_json: get("LOG_FORMAT").is_some_and(|v| v.eq_ignore_ascii_case("json")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(pairs: &[(&str, &str)]) -> Result<Config> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    fn ceilings() -> Ceilings {
        cfg(&[]).unwrap().ceilings
    }

    #[test]
    fn defaults_match_the_services_contract() {
        let c = cfg(&[]).unwrap();
        assert_eq!(c.bind, "0.0.0.0:8081");
        assert_eq!(c.work_dir, PathBuf::from("/tmp/dsa-runner"));
        assert_eq!(c.ceilings.run_timeout, Duration::from_secs(10));
        assert_eq!(c.ceilings.compile_timeout, Duration::from_secs(30));
        assert_eq!(c.ceilings.memory_mb, 512);
        assert!(c.token.is_none());
        assert!(!c.allow_unsandboxed);
        assert!(c.max_concurrency >= 1);
    }

    #[test]
    fn requests_can_never_exceed_the_ceiling() {
        let huge = Limits {
            compile_timeout_ms: u64::MAX,
            run_timeout_ms: u64::MAX,
            total_timeout_ms: u64::MAX,
            memory_mb: u64::MAX,
            max_output_bytes: u64::MAX,
        };
        let c = ceilings();
        let e = c.clamp(&huge);
        assert_eq!(e.run_timeout, c.run_timeout);
        assert_eq!(e.compile_timeout, c.compile_timeout);
        assert_eq!(e.total_timeout, c.total_timeout);
        assert_eq!(e.memory_mb, c.memory_mb);
        assert_eq!(e.max_output_bytes as u64, c.max_output_bytes);
    }

    #[test]
    fn zero_limits_are_raised_to_a_useful_floor() {
        let zero = Limits {
            compile_timeout_ms: 0,
            run_timeout_ms: 0,
            total_timeout_ms: 0,
            memory_mb: 0,
            max_output_bytes: 0,
        };
        let e = ceilings().clamp(&zero);
        assert_eq!(e.run_timeout, MIN_RUN_TIMEOUT);
        assert_eq!(e.compile_timeout, MIN_COMPILE_TIMEOUT);
        assert_eq!(e.total_timeout, MIN_TOTAL_TIMEOUT);
        assert_eq!(e.memory_mb, MIN_MEMORY_MB);
        assert_eq!(e.max_output_bytes as u64, MIN_OUTPUT_BYTES);
    }

    #[test]
    fn reasonable_requests_pass_through_unchanged() {
        let e = ceilings().clamp(&Limits::default());
        let d = Limits::default();
        assert_eq!(e.run_timeout, Duration::from_millis(d.run_timeout_ms));
        assert_eq!(
            e.compile_timeout,
            Duration::from_millis(d.compile_timeout_ms)
        );
        assert_eq!(e.total_timeout, Duration::from_millis(d.total_timeout_ms));
        assert_eq!(e.memory_mb, d.memory_mb);
        assert_eq!(e.max_output_bytes as u64, d.max_output_bytes);
    }

    #[test]
    fn the_compiler_always_gets_at_least_the_program_budget() {
        let mut c = ceilings();
        c.compile_memory_mb = 64;
        let e = c.clamp(&Limits {
            memory_mb: 300,
            ..Limits::default()
        });
        assert_eq!(e.memory_mb, 300);
        assert_eq!(e.compile_memory_mb, 300);
    }

    #[test]
    fn a_misconfigured_ceiling_below_the_floor_still_clamps_sanely() {
        let c = cfg(&[
            ("RUNNER_MAX_RUN_TIMEOUT_MS", "10"),
            ("RUNNER_MAX_MEMORY_MB", "1"),
        ])
        .unwrap()
        .ceilings;
        let e = c.clamp(&Limits::default());
        assert_eq!(e.run_timeout, MIN_RUN_TIMEOUT);
        assert_eq!(e.memory_mb, MIN_MEMORY_MB);
    }

    #[test]
    fn environment_overrides_and_rejects_garbage() {
        let c = cfg(&[
            ("RUNNER_MAX_CONCURRENCY", "3"),
            ("RUNNER_TOKEN", "  s3cret  "),
            ("RUNNER_ALLOW_UNSANDBOXED", "true"),
            ("LOG_FORMAT", "JSON"),
            ("RUNNER_MAX_MEMORY_MB", "1024"),
        ])
        .unwrap();
        assert_eq!(c.max_concurrency, 3);
        assert_eq!(c.token.as_deref(), Some("s3cret"));
        assert!(c.allow_unsandboxed);
        assert!(c.log_json);
        assert_eq!(c.ceilings.memory_mb, 1024);

        assert!(cfg(&[("RUNNER_MAX_CONCURRENCY", "0")]).is_err());
        assert!(cfg(&[("RUNNER_MAX_CONCURRENCY", "lots")]).is_err());
        assert!(cfg(&[("RUNNER_MAX_MEMORY_MB", "-5")]).is_err());
        assert!(cfg(&[("RUNNER_MAX_CONCURRENCY", "100000")]).is_err());
        // An empty token is no token, not a token that matches empty headers.
        assert!(cfg(&[("RUNNER_TOKEN", "   ")]).unwrap().token.is_none());
        for shared in ["/", "/tmp", "/var/tmp", "/dev/shm"] {
            assert!(cfg(&[("RUNNER_WORK_DIR", shared)]).is_err(), "{shared}");
        }
    }

    #[test]
    fn the_default_body_limit_admits_the_largest_valid_request() {
        assert!(default_max_body_bytes() > MAX_SOURCE_BYTES + MAX_CASES * MAX_STDIN_BYTES);
    }
}
