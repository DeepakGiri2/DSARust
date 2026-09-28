//! The Linux sandbox: confinement between `fork` and `exec`, a resident-memory
//! watchdog, and the reaper that makes sure nothing outlives its job.
//!
//! The runner has no namespaces or cgroups to work with (an unprivileged
//! container has neither `CAP_SYS_ADMIN` nor a writable cgroup tree, and Lambda
//! offers neither), so confinement is built from what any process may do to
//! itself before `exec`, in this order:
//!
//! 1. `setsid` — own session and process group, so one `kill(-pgid)` reaches
//!    every descendant that did not leave it (the reaper handles those).
//! 2. Every inherited descriptor above stderr is marked close-on-exec, so the
//!    program starts with exactly stdin, stdout and stderr.
//! 3. rlimits — CPU, file size, descriptors, stack, core dumps off, threads
//!    (see [`process_limit`]) and address space (when the language tolerates
//!    it).
//! 4. `PR_SET_NO_NEW_PRIVS` — `exec` can never grant privileges again: setuid
//!    binaries and file capabilities are inert from here on.
//! 5. Drop to the job's uid/gid with supplementary groups cleared (root mode),
//!    and verify root cannot be regained.
//! 6. `chdir` into the case directory — after the drop, so it works even
//!    without `CAP_DAC_OVERRIDE` and proves the job can reach its own files.
//! 7. The seccomp filter (see [`super::seccomp`]) — last, so it constrains the
//!    program and nothing the runner itself still needed.
//!
//! The program is deliberately *not* made non-dumpable: `exec` recomputes
//! dumpability for the new image (a readable, non-setuid binary is dumpable
//! again), so a `PR_SET_DUMPABLE` here would not survive it. Nothing needs it
//! to: what keeps other processes out of a job is that none share its uid —
//! a private uid in root mode, one job at a time otherwise. (The *runner* is
//! non-dumpable, set once at startup; see `hardening`.)
//!
//! Everything in `Confinement::apply` runs in the forked child of a
//! multi-threaded parent, where only async-signal-safe calls are allowed: it
//! makes raw syscalls on values captured before the fork and never allocates
//! (an allocator lock held by another parent thread at fork time would
//! deadlock the child).

use super::{
    cpu_seconds, seccomp, supervise, ExecResult, ExecSpec, ReapScope, FILE_SIZE_LIMIT,
    OPEN_FILES_LIMIT, PROCESS_LIMIT, STACK_LIMIT,
};
use crate::slots::Credentials;
use seccompiler::BpfProgram;
use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::time::MissedTickBehavior;

/// How often the memory watchdog samples the process group. A program that
/// allocates as fast as it can touches roughly 1–2 GiB/s, so this bounds the
/// overshoot past the limit to a few tens of MiB before the kill.
const MEMORY_POLL: Duration = Duration::from_millis(20);

/// Reaper passes: each kills what it can see; the next catches children that
/// were forked, or re-parented to the runner, in the meantime.
const REAP_PASSES: usize = 50;
const REAP_PAUSE: Duration = Duration::from_millis(2);

/// World-writable directories a job can create files in despite never being
/// told about them. Its own directory is removed separately; these are swept
/// so nothing one job leaves behind is readable by the next. `/dev/mqueue` is
/// the POSIX message-queue filesystem Docker mounts: `open(O_CREAT)` there
/// creates a queue even though `mq_open` itself is refused by seccomp, and a
/// queue outlives its creator like a file does.
const SHARED_TMP_DIRS: &[&str] = &["/tmp", "/var/tmp", "/dev/shm", "/dev/mqueue"];

/// `close_range(2)` flag (Linux 5.11): mark the range close-on-exec instead
/// of closing it now. Not closing matters: std reports a failed `exec` back to
/// the parent through a close-on-exec pipe that is open at this point.
const CLOSE_RANGE_CLOEXEC: libc::c_uint = 1 << 2;
/// Fallback scan bound for kernels without `close_range`. Every descriptor
/// std and tokio create is already close-on-exec; this only has to catch a
/// stray inherited one, which would be a low number.
const FALLBACK_FD_SCAN: libc::c_int = 4096;

#[cfg(target_env = "gnu")]
type Resource = libc::__rlimit_resource_t;
#[cfg(not(target_env = "gnu"))]
type Resource = libc::c_int;

/// The compiled seccomp filter, built once at startup and shared by every
/// process start.
#[derive(Debug)]
pub struct Sandbox {
    filter: Arc<BpfProgram>,
}

impl Sandbox {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            filter: Arc::new(seccomp::build()?),
        })
    }
}

/// Everything the child needs between `fork` and `exec`, captured by value.
struct Confinement {
    cwd: CString,
    cpu_seconds: u64,
    address_space: Option<u64>,
    processes: Option<u64>,
    credentials: Option<Credentials>,
    filter: Arc<BpfProgram>,
}

impl Confinement {
    fn apply(&self) -> io::Result<()> {
        let last = io::Error::last_os_error;
        // SAFETY: we are in the child between fork and exec. Each call below is
        // a thin syscall wrapper operating on memory captured before the fork;
        // none allocates or takes a lock.
        unsafe {
            if libc::setsid() < 0 {
                return Err(last());
            }
            mark_inherited_fds_cloexec();

            limit(libc::RLIMIT_CORE, 0, 0)?;
            // Soft limit → SIGXCPU (reported as a timeout); hard limit one
            // second later → SIGKILL for a program that ignores SIGXCPU.
            limit(libc::RLIMIT_CPU, self.cpu_seconds, self.cpu_seconds + 1)?;
            limit(libc::RLIMIT_FSIZE, FILE_SIZE_LIMIT, FILE_SIZE_LIMIT)?;
            limit(libc::RLIMIT_NOFILE, OPEN_FILES_LIMIT, OPEN_FILES_LIMIT)?;
            limit(libc::RLIMIT_STACK, STACK_LIMIT, STACK_LIMIT)?;
            if let Some(n) = self.processes {
                limit(libc::RLIMIT_NPROC, n, n)?;
            }
            if let Some(bytes) = self.address_space {
                limit(libc::RLIMIT_AS, bytes, bytes)?;
            }

            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(last());
            }

            if let Some(c) = self.credentials {
                // Order matters: groups and gid while still root, uid last.
                if libc::setgroups(0, std::ptr::null()) != 0
                    || libc::setgid(c.gid) != 0
                    || libc::setuid(c.uid) != 0
                {
                    return Err(last());
                }
                // Belt and braces: the drop must be complete and irreversible.
                if libc::getuid() != c.uid
                    || libc::geteuid() != c.uid
                    || libc::getgid() != c.gid
                    || libc::setuid(0) == 0
                {
                    return Err(io::Error::from_raw_os_error(libc::EPERM));
                }
            }

            if libc::chdir(self.cwd.as_ptr()) != 0 {
                return Err(last());
            }
            seccompiler::apply_filter(&self.filter)
                .map_err(|_| io::Error::from_raw_os_error(libc::EPERM))?;
        }
        Ok(())
    }
}

/// Set an rlimit, settling for the current hard limit when raising it is not
/// allowed (raising a hard limit needs `CAP_SYS_RESOURCE`, which containers
/// do not get). The result is never *looser* than asked.
unsafe fn limit(resource: Resource, soft: u64, hard: u64) -> io::Result<()> {
    let mut want = libc::rlimit {
        rlim_cur: soft as libc::rlim_t,
        rlim_max: hard as libc::rlim_t,
    };
    if libc::setrlimit(resource, &want) == 0 {
        return Ok(());
    }
    let mut current = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if libc::getrlimit(resource, &mut current) != 0 {
        return Err(io::Error::last_os_error());
    }
    want.rlim_max = want.rlim_max.min(current.rlim_max);
    want.rlim_cur = want.rlim_cur.min(want.rlim_max);
    if libc::setrlimit(resource, &want) == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

unsafe fn mark_inherited_fds_cloexec() {
    // `syscall` is variadic (each argument widened to `long`), so pass the
    // arguments at that width: a bare `u32::MAX` could reach the kernel with
    // garbage in its high bits and mark the wrong range.
    let all = libc::syscall(
        libc::SYS_close_range,
        3 as libc::c_long,
        libc::c_uint::MAX as libc::c_long,
        CLOSE_RANGE_CLOEXEC as libc::c_long,
    );
    if all == 0 {
        return;
    }
    for fd in 3..FALLBACK_FD_SCAN {
        libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
    }
}

/// Start `spec` confined, and supervise it to the end.
pub async fn run(spec: ExecSpec, sandbox: &Sandbox) -> io::Result<ExecResult> {
    let (program, args) = spec
        .argv
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty argv"))?;
    let confinement = Confinement {
        cwd: CString::new(spec.cwd.as_os_str().as_bytes())?,
        cpu_seconds: cpu_seconds(spec.wall),
        address_space: spec.memory.address_space,
        processes: spec.processes,
        credentials: spec.credentials,
        filter: Arc::clone(&sandbox.filter),
    };

    let mut cmd = std::process::Command::new(program);
    // With `PATH` in the child's environment, std resolves `program` against
    // that `PATH`, not the runner's.
    cmd.args(args)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: `Confinement::apply` is async-signal-safe (see its comment).
    unsafe {
        cmd.pre_exec(move || confinement.apply());
    }
    let mut cmd = tokio::process::Command::from(cmd);
    cmd.kill_on_drop(true);

    let started = Instant::now();
    let child = cmd.spawn()?;
    // `setsid` made the child its own group leader: pgid == pid.
    let pgid = child
        .id()
        .and_then(|pid| libc::pid_t::try_from(pid).ok())
        .ok_or_else(|| io::Error::other("the child has no pid"))?;
    let scope = spec.reap;
    supervise(
        child,
        &spec,
        started,
        watch_memory(pgid, spec.memory.resident),
        move || kill_group(pgid),
        move || reap(scope),
    )
    .await
}

/// `RLIMIT_NPROC` for a job.
///
/// With its own uid the job starts from zero threads, so the budget is the
/// limit. Sharing the runner's uid, the count already includes the runner's
/// own threads (and, in development, the developer's processes), so the job
/// gets the budget *on top of* what exists now. The limit is set only in the
/// child; the runner's own (unlimited) rlimit, which is what its own thread
/// creation is checked against, is untouched.
pub fn process_limit(credentials: Option<Credentials>) -> Option<u64> {
    if credentials.is_some() {
        return Some(PROCESS_LIMIT);
    }
    // SAFETY: getuid has no preconditions.
    let uid = unsafe { libc::getuid() };
    let existing: u64 = pids()
        .filter_map(ProcStatus::read)
        .filter(|p| p.uid == uid)
        .map(|p| p.threads)
        .sum();
    Some(existing + PROCESS_LIMIT)
}

fn kill_group(pgid: libc::pid_t) {
    // SAFETY: plain syscall; a negative pid addresses the process group.
    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
    }
}

/// Resolves once the process group's resident memory exceeds `limit`.
async fn watch_memory(pgid: libc::pid_t, limit: u64) {
    let mut tick = tokio::time::interval(MEMORY_POLL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        if group_resident_bytes(pgid) > limit {
            return;
        }
    }
}

fn page_size() -> u64 {
    static PAGE: OnceLock<u64> = OnceLock::new();
    // SAFETY: sysconf has no preconditions.
    *PAGE
        .get_or_init(|| u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap_or(4096))
}

/// Sum of RSS over every process in the group. Pages shared between processes
/// are counted once per process, which over-reports for fork-heavy programs:
/// the error is in the safe direction. Reading `/proc` is not disk I/O, so
/// doing it on the async runtime every 20 ms is cheap.
fn group_resident_bytes(pgid: libc::pid_t) -> u64 {
    let pages: u64 = pids()
        .filter_map(|pid| {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            // `comm` (field 2) may contain spaces and parentheses; everything
            // after the *last* ')' is space-separated, starting at field 3.
            let mut fields = stat[stat.rfind(')')? + 1..].split_whitespace();
            let pgrp: libc::pid_t = fields.nth(2)?.parse().ok()?; // field 5
            let rss: u64 = fields.nth(18)?.parse().ok()?; // field 24
            (pgrp == pgid).then_some(rss)
        })
        .sum();
    pages * page_size()
}

fn pids() -> impl Iterator<Item = libc::pid_t> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<libc::pid_t>().ok())
        .filter(|&pid| pid > 0)
}

struct ProcStatus {
    ppid: libc::pid_t,
    uid: u32,
    threads: u64,
    zombie: bool,
}

impl ProcStatus {
    fn read(pid: libc::pid_t) -> Option<Self> {
        let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let field = |name: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(name))
                .map(str::trim_start)
        };
        Some(Self {
            ppid: field("PPid:")?.parse().ok()?,
            // Real uid: the first of real/effective/saved/filesystem.
            uid: field("Uid:")?.split_whitespace().next()?.parse().ok()?,
            // A process caught mid-exit may have lost its `Threads:` line.
            threads: field("Threads:").and_then(|t| t.parse().ok()).unwrap_or(0),
            zombie: field("State:")?.starts_with('Z'),
        })
    }
}

fn ancestors_of(mut pid: libc::pid_t) -> Vec<libc::pid_t> {
    let mut chain = Vec::new();
    while pid > 1 {
        let Some(parent) = ProcStatus::read(pid).map(|s| s.ppid) else {
            break;
        };
        chain.push(parent);
        pid = parent;
    }
    chain
}

/// Kill every process in `scope` and reap the ones that are (or became) the
/// runner's children, until a pass finds nothing left.
///
/// A daemon that double-forked and called `setsid` is out of reach of the
/// group kill, but it still runs as the job's uid (root mode) or is re-parented
/// to the runner, which is a child subreaper (every mode) — so it is found
/// here, and cannot survive into the next job.
pub fn reap(scope: ReapScope) {
    let me = libc::pid_t::try_from(std::process::id()).unwrap_or(libc::pid_t::MAX);
    // SAFETY: getuid has no preconditions.
    let my_uid = unsafe { libc::getuid() };
    let spared = match scope {
        ReapScope::AllButAncestors => ancestors_of(me),
        ReapScope::Uid(_) | ReapScope::Descendants => Vec::new(),
    };
    for _ in 0..REAP_PASSES {
        let mut acted = false;
        for pid in pids() {
            if pid == me || spared.contains(&pid) {
                continue;
            }
            let Some(p) = ProcStatus::read(pid) else {
                continue;
            };
            let in_scope = match scope {
                ReapScope::Uid(uid) => p.uid == uid,
                ReapScope::AllButAncestors => p.uid == my_uid,
                ReapScope::Descendants => p.ppid == me,
            };
            if !in_scope {
                continue;
            }
            acted = true;
            // SAFETY: plain syscalls on a pid we just observed. Only zombies
            // in scope are waited for, so a concurrent job's child — which
            // tokio is waiting on — is never reaped from under it.
            unsafe {
                if !p.zombie {
                    libc::kill(pid, libc::SIGKILL);
                } else if p.ppid == me {
                    libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG);
                }
            }
        }
        if !acted {
            return;
        }
        std::thread::sleep(REAP_PAUSE);
    }
}

/// Remove what a job left in the shared temp directories.
///
/// Root mode removes entries owned by the job's uid (a job can only create
/// entries in world-writable directories, so the walk only descends into
/// those). Lambda removes everything except the runner's own work root: the
/// execution environment is the runner's alone, but `/tmp` persists between
/// invocations, i.e. between users. Unprivileged development never sweeps a
/// developer's `/tmp`.
pub fn sweep_shared_tmp(scope: ReapScope, keep: &Path) {
    for root in SHARED_TMP_DIRS {
        match scope {
            ReapScope::Uid(uid) => sweep_owned(Path::new(root), uid, keep, 3),
            ReapScope::AllButAncestors => sweep_all(Path::new(root), keep),
            ReapScope::Descendants => return,
        }
    }
}

fn sweep_owned(dir: &Path, uid: u32, keep: &Path, depth: u32) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if keep.starts_with(&path) {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.uid() == uid {
            remove(&path, &meta);
        } else if depth > 0 && meta.is_dir() && meta.mode() & 0o002 != 0 {
            sweep_owned(&path, uid, keep, depth - 1);
        }
    }
}

fn sweep_all(dir: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if keep.starts_with(&path) {
            continue;
        }
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            remove(&path, &meta);
        }
    }
}

/// `remove_dir_all` never follows symlinks (it works through `openat` with
/// `O_NOFOLLOW`), so a hostile tree cannot redirect the deletion elsewhere.
fn remove(path: &Path, meta: &std::fs::Metadata) {
    let _ = if meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
}
