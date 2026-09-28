//! Starting one process and watching it until it ends.
//!
//! [`run`] is the only way the runner starts user-influenced code (a compiler
//! fed a user's source counts: `g++` on a hostile file is hostile input to
//! `g++`). On Linux it confines the process before `exec` (see `linux`); on
//! other platforms it runs it plainly, for development only.
//!
//! Both variants share [`supervise`], which feeds stdin, drains stdout and
//! stderr under a byte cap, and races the process against three watchdogs —
//! wall clock, output size and (on Linux) resident memory. Whichever fires
//! first decides the [`Termination`], and the whole process group dies with
//! it.

use crate::classify::Termination;
use crate::slots::Credentials;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{mpsc, watch};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod portable;
#[cfg(target_os = "linux")]
pub mod seccomp;

#[cfg(target_os = "linux")]
pub use linux::{process_limit, reap, run, sweep_shared_tmp, Sandbox};
#[cfg(not(target_os = "linux"))]
pub use portable::{process_limit, reap, run, sweep_shared_tmp, Sandbox};

/// `RLIMIT_FSIZE`: the largest file a job may write. Problems never need to
/// write files at all; this only stops a job from filling the disk that the
/// next job (and the runner's own scratch space) lives on.
pub const FILE_SIZE_LIMIT: u64 = 16 * 1024 * 1024;

/// `RLIMIT_NOFILE`. Generous for any real solution (the JVM opens ~30 files
/// at startup), small enough that descriptor exhaustion stays local.
pub const OPEN_FILES_LIMIT: u64 = 256;

/// Threads a job may add. `RLIMIT_NPROC` counts *threads*, not processes,
/// across every process of the uid: the JVM starts ~20 threads before `main`,
/// the Go runtime one per busy `P` plus sysmon, and `go build` runs compile
/// and link concurrently. 256 is comfortably above all of that, and still
/// stops a fork bomb within milliseconds.
pub const PROCESS_LIMIT: u64 = 256;

/// `RLIMIT_STACK` for the main thread. DSA solutions recurse (DFS on a 10⁵
/// node path graph needs tens of MiB in C++), and the default 8 MiB turns a
/// correct answer into a segfault.
pub const STACK_LIMIT: u64 = 64 * 1024 * 1024;

/// How long to keep draining a stream after the process group is dead. The
/// pipes close when the last writer dies, so this only matters when some
/// descendant escaped the group kill and the reaper has not caught it yet.
const OUTPUT_GRACE: Duration = Duration::from_millis(500);

/// Memory policy for one process (see `registry` for why it differs by
/// language).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryLimit {
    /// `RLIMIT_AS`, when the runtime tolerates one.
    pub address_space: Option<u64>,
    /// Resident-set ceiling for the whole process group, enforced by polling.
    pub resident: u64,
}

/// Everything about one process start. Built by the engine from the language
/// registry and the clamped limits — never from the request directly.
#[derive(Clone, Debug)]
pub struct ExecSpec {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    /// The *complete* environment: the child inherits nothing else.
    pub env: Vec<(String, String)>,
    /// Written to the child's stdin, which is then closed.
    pub stdin: Vec<u8>,
    pub wall: Duration,
    pub memory: MemoryLimit,
    /// Per stream (stdout, stderr).
    pub max_output_bytes: usize,
    /// Identity to drop to before `exec` (root mode), or `None`.
    pub credentials: Option<Credentials>,
    /// `RLIMIT_NPROC` (see [`process_limit`]).
    pub processes: Option<u64>,
    /// What to kill once the process has ended.
    pub reap: ReapScope,
}

/// How a process ended and what it printed.
#[derive(Clone, Debug)]
pub struct ExecResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub duration: Duration,
    pub termination: Termination,
}

/// Which processes the reaper may kill once a case or job is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReapScope {
    /// Root mode: every process of the job's private uid, wherever it is in
    /// the process tree and whatever session it moved itself into.
    Uid(u32),
    /// Lambda: jobs share the runner's uid, but the microVM is ours alone and
    /// runs one job at a time, so every same-uid process that is neither the
    /// runner nor one of its ancestors belongs to the job.
    AllButAncestors,
    /// Unprivileged elsewhere (development): only processes that ended up as
    /// the runner's direct children. The runner is a child subreaper, so
    /// double-forked daemons are re-parented to it and land here too, while a
    /// developer's unrelated processes are never touched.
    Descendants,
}

/// `RLIMIT_CPU` for a wall-clock budget: one second of slack, so the precise
/// wall-clock watchdog normally fires first and this only backstops a runner
/// that is itself too starved to notice.
pub fn cpu_seconds(wall: Duration) -> u64 {
    wall.as_secs() + u64::from(wall.subsec_nanos() > 0) + 1
}

/// Read a stream into memory, stopping at `cap` bytes.
///
/// Returns what was kept and whether the stream had more. The first byte past
/// the cap signals `overflow`, which makes [`supervise`] kill the process
/// group; reading stops then too, so a flood costs at most `cap` bytes of
/// memory however much the program writes. `stop` abandons the read (keeping
/// what was read) when a stray descendant keeps the pipe open.
async fn read_capped<R: AsyncRead + Unpin>(
    mut reader: R,
    cap: usize,
    overflow: mpsc::Sender<()>,
    mut stop: watch::Receiver<bool>,
) -> (Vec<u8>, bool) {
    let mut kept = Vec::with_capacity(cap.min(16 * 1024));
    let mut chunk = vec![0u8; 16 * 1024];
    loop {
        let n = tokio::select! {
            // Prefer data over the stop signal so nothing already in the pipe
            // is dropped on a race.
            biased;
            read = reader.read(&mut chunk) => match read {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            },
            _ = stop.changed() => break,
        };
        let room = cap - kept.len();
        if n > room {
            kept.extend_from_slice(&chunk[..room]);
            let _ = overflow.try_send(());
            return (kept, true);
        }
        kept.extend_from_slice(&chunk[..n]);
    }
    (kept, false)
}

/// Write the case input and close stdin. A program that never reads its input
/// makes this fail with a broken pipe once it exits, which is not an error.
async fn feed_stdin(stdin: Option<ChildStdin>, data: Vec<u8>) {
    if let Some(mut pipe) = stdin {
        let _ = pipe.write_all(&data).await;
        let _ = pipe.shutdown().await;
    }
}

/// Watch a freshly spawned child until it ends or a watchdog stops it.
///
/// * `memory_exceeded` resolves only if the group goes over its memory limit
///   (pass `std::future::pending()` where that cannot be measured).
/// * `kill_group` kills every process in the child's group. It runs on *every*
///   path, including a clean exit: a program's background children must not
///   outlive its case, and while they live they hold stdout open.
/// * `reap_escapees` kills whatever left the group (`setsid` in the child),
///   again because such a process can hold the pipes open indefinitely.
async fn supervise(
    mut child: Child,
    spec: &ExecSpec,
    started: Instant,
    memory_exceeded: impl Future<Output = ()>,
    kill_group: impl Fn(),
    reap_escapees: impl Fn(),
) -> io::Result<ExecResult> {
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let cap = spec.max_output_bytes;

    let (overflow_tx, mut overflow_rx) = mpsc::channel(2);
    let (stop_tx, stop_rx) = watch::channel(false);
    let out = tokio::spawn(read_capped(
        stdout.ok_or_else(|| io::Error::other("stdout was not piped"))?,
        cap,
        overflow_tx.clone(),
        stop_rx.clone(),
    ));
    let err = tokio::spawn(read_capped(
        stderr.ok_or_else(|| io::Error::other("stderr was not piped"))?,
        cap,
        overflow_tx,
        stop_rx,
    ));
    let feeder = tokio::spawn(feed_stdin(stdin, spec.stdin.clone()));

    let deadline = tokio::time::sleep(spec.wall);
    tokio::pin!(deadline, memory_exceeded);
    let (termination, waited) = tokio::select! {
        status = child.wait() => (Termination::Exited, Some(status)),
        () = &mut deadline => (Termination::TimedOut, None),
        // `None` means both readers finished without overflowing: that branch
        // is then disabled and the others keep racing.
        Some(()) = overflow_rx.recv() => (Termination::OutputExceeded, None),
        () = &mut memory_exceeded => (Termination::MemoryExceeded, None),
    };
    let duration = started.elapsed();

    // After a clean exit the group leader is already reaped, so its pgid could
    // in principle be recycled before this signal; that needs the kernel to
    // wrap the whole pid space (4M on 64-bit) in microseconds. Signalling
    // first and reaping second is not possible through tokio's `Child`.
    kill_group();
    let status = match waited {
        Some(status) => status,
        None => {
            let _ = child.start_kill();
            child.wait().await
        }
    };
    reap_escapees();

    let drain = async { (out.await, err.await) };
    tokio::pin!(drain);
    let (out, err) = match tokio::time::timeout(OUTPUT_GRACE, &mut drain).await {
        Ok(done) => done,
        Err(_) => {
            let _ = stop_tx.send(true);
            drain.await
        }
    };
    feeder.abort();

    let status = status?;
    let (stdout, stdout_truncated) = out.unwrap_or_default();
    let (stderr, stderr_truncated) = err.unwrap_or_default();
    Ok(ExecResult {
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
        exit_code: status.code(),
        signal: exit_signal(&status),
        duration,
        // A stream that overflowed while the process was exiting anyway still
        // exceeded the limit.
        termination: match termination {
            Termination::Exited if stdout_truncated || stderr_truncated => {
                Termination::OutputExceeded
            }
            other => other,
        },
    })
}

#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_limit_always_trails_the_wall_clock() {
        assert_eq!(cpu_seconds(Duration::from_secs(5)), 6);
        assert_eq!(cpu_seconds(Duration::from_millis(5_001)), 7);
        assert_eq!(cpu_seconds(Duration::from_millis(100)), 2);
        for ms in [1, 999, 1_000, 10_000, 29_999] {
            let wall = Duration::from_millis(ms);
            assert!(Duration::from_secs(cpu_seconds(wall)) > wall, "{ms} ms");
        }
    }

    fn channels() -> (
        mpsc::Sender<()>,
        mpsc::Receiver<()>,
        watch::Sender<bool>,
        watch::Receiver<bool>,
    ) {
        let (o_tx, o_rx) = mpsc::channel(2);
        let (s_tx, s_rx) = watch::channel(false);
        (o_tx, o_rx, s_tx, s_rx)
    }

    #[tokio::test]
    async fn output_under_the_cap_is_kept_whole() {
        let (o_tx, mut o_rx, _s_tx, s_rx) = channels();
        let (kept, cut) = read_capped(&b"0 1\n"[..], 16, o_tx, s_rx).await;
        assert_eq!(kept, b"0 1\n");
        assert!(!cut);
        assert!(o_rx.recv().await.is_none(), "no overflow signalled");
    }

    #[tokio::test]
    async fn exactly_the_cap_is_not_an_overflow() {
        let (o_tx, _o_rx, _s_tx, s_rx) = channels();
        let (kept, cut) = read_capped(&b"abcd"[..], 4, o_tx, s_rx).await;
        assert_eq!(kept, b"abcd");
        assert!(!cut);
    }

    #[tokio::test]
    async fn a_flood_is_cut_at_the_cap_and_signalled() {
        let (o_tx, mut o_rx, _s_tx, s_rx) = channels();
        let flood = vec![b'y'; 100_000];
        let (kept, cut) = read_capped(&flood[..], 1000, o_tx, s_rx).await;
        assert_eq!(kept.len(), 1000);
        assert!(cut);
        assert_eq!(o_rx.recv().await, Some(()));
    }

    #[tokio::test]
    async fn a_stuck_stream_can_be_abandoned_without_losing_what_was_read() {
        let (o_tx, _o_rx, s_tx, s_rx) = channels();
        let (mut writer, reader) = tokio::io::duplex(64);
        writer.write_all(b"partial").await.unwrap();
        // `writer` stays open: the read would block forever without `stop`.
        let task = tokio::spawn(read_capped(reader, 64, o_tx, s_rx));
        tokio::time::sleep(Duration::from_millis(20)).await;
        s_tx.send(true).unwrap();
        let (kept, cut) = task.await.unwrap();
        assert_eq!(kept, b"partial");
        assert!(!cut);
        drop(writer);
    }
}
