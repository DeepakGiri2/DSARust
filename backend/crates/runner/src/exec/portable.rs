//! The unsandboxed executor for non-Linux development machines.
//!
//! It exists so the whole request path can be exercised on a Windows or macOS
//! checkout, and it is reachable only when `RUNNER_ALLOW_UNSANDBOXED=1` (the
//! engine refuses jobs otherwise). It confines nothing: no uid switch, no
//! rlimits, no seccomp, no memory watchdog. The wall clock and the output cap
//! still apply, because those are what keep a developer's machine usable.

use super::{supervise, ExecResult, ExecSpec, ReapScope};
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

/// Nothing to hold: there is no filter to install.
#[derive(Debug)]
pub struct Sandbox;

impl Sandbox {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}

pub async fn run(spec: ExecSpec, _sandbox: &Sandbox) -> io::Result<ExecResult> {
    let (program, args) = spec
        .argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty argv"))?;

    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .envs(platform_env(&spec.cwd))
        .current_dir(&spec.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // No console window flashing up for every compile.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let started = Instant::now();
    let child = cmd.spawn()?;
    supervise(child, &spec, started, std::future::pending(), || {}, || {}).await
}

/// Variables the OS itself needs for a process to start at all. Windows
/// loads system DLLs relative to `SystemRoot`, and toolchains look up their
/// temp and profile directories through these rather than `HOME`/`TMPDIR`.
fn platform_env(scratch: &Path) -> Vec<(String, String)> {
    let scratch = scratch.to_string_lossy().into_owned();
    let mut env = Vec::new();
    if cfg!(windows) {
        for key in [
            "SystemRoot",
            "windir",
            "PATHEXT",
            "ComSpec",
            "NUMBER_OF_PROCESSORS",
        ] {
            if let Ok(v) = std::env::var(key) {
                env.push((key.to_string(), v));
            }
        }
        for key in ["TEMP", "TMP", "USERPROFILE", "LOCALAPPDATA", "APPDATA"] {
            env.push((key.to_string(), scratch.clone()));
        }
    }
    env
}

/// No rlimits without the Linux sandbox.
pub fn process_limit(_credentials: Option<crate::slots::Credentials>) -> Option<u64> {
    None
}

/// Nothing is tracked, so nothing can be reaped beyond the process group
/// `supervise` already killed.
pub fn reap(_scope: ReapScope) {}

/// Development machines keep their shared temp directories to themselves.
pub fn sweep_shared_tmp(_scope: ReapScope, _keep: &Path) {}
