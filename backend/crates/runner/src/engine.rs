//! One job, start to finish: validate, clamp, lay out, compile once, run each
//! case in a fresh directory, tear everything down.
//!
//! ## Filesystem layout and ownership (root mode)
//!
//! ```text
//! $RUNNER_WORK_DIR/            root:root 0711  jobs can traverse, not list or write
//!   job-<random>/              root:job  0750  the job reads, cannot write
//!     main.go                  root:job  0640
//!     tmp/                     job:job   0700  compile $HOME/$TMPDIR   (removed after build)
//!     gocache/ gopath/         job:job   0700  Go build state          (removed after build)
//!     bin/prog | classes/      job:job → root:job, read-only after the build ("sealed")
//!     cases/                   root:job  0750
//!       0/ 1/ …                job:job   0700  one case's cwd/$HOME/$TMPDIR, removed after it
//! ```
//!
//! The only places a *case* can write are its own directory and the shared
//! world-writable temp dirs, and both are wiped before the next case starts:
//! nothing one case leaves behind is visible to the next. The directory name
//! is random rather than the caller's `job_id`, so no request can steer a
//! path.
//!
//! Without root (Lambda) every process shares the runner's uid, so ownership
//! cannot separate the runner from the job or one case from the next. There
//! the runner executes one job at a time, and the teardown between jobs —
//! kill everything, recreate the work root, wipe `/tmp` — is the boundary
//! between users.

use crate::classify::{classify, Termination};
use crate::config::{Config, EffectiveLimits};
use crate::exec::{self, ExecResult, ExecSpec, ReapScope, Sandbox};
use crate::output;
use crate::registry::{child_path, Language, Layout};
use crate::slots::{Credentials, SlotGuard, SlotPool};
use crate::RUNNER_VERSION;
use dsa_protocol::{
    CaseInput, CaseOutcome, CaseStatus, CompileOutcome, ExecuteRequest, ExecuteResponse,
    LanguageInfo, RunnerError, RunnerErrorCode, RunnerHealth, PROTOCOL_VERSION,
};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A case that would get less than this is skipped rather than started only
/// to be reported as a timeout the program never had a chance to avoid.
const MIN_CASE_SLICE: Duration = Duration::from_millis(50);

/// The executor for the whole process: configuration, the toolchains found at
/// startup, the slot pool and the compiled sandbox.
pub struct Engine {
    config: Config,
    languages: BTreeMap<Language, String>,
    pool: SlotPool,
    /// `None` when this runner refuses to execute anything (non-Linux without
    /// `RUNNER_ALLOW_UNSANDBOXED`); the reason is in `refusal`.
    sandbox: Option<Sandbox>,
    refusal: String,
    /// Lambda mode: the runner owns its execution environment outright.
    lambda: bool,
}

impl Engine {
    /// Probe toolchains, compile the sandbox and prepare the work directory.
    /// Errors here are fatal: a runner that cannot sandbox must not serve.
    pub fn new(config: Config, lambda: bool) -> anyhow::Result<Self> {
        let root = crate::hardening::is_root();
        // Without a uid per job, two concurrent jobs would share a uid and
        // could read and signal each other; serialising them is the only
        // isolation left.
        let capacity = if root { config.max_concurrency } else { 1 };
        if capacity < config.max_concurrency {
            tracing::warn!(
                requested = config.max_concurrency,
                "not running as root: jobs cannot get their own uid, so they run one at a time"
            );
        }

        let languages = probe_languages();
        for lang in Language::ALL {
            match languages.get(&lang) {
                Some(v) => tracing::info!(language = lang.id(), version = %v, "toolchain found"),
                None => {
                    tracing::warn!(language = lang.id(), "toolchain missing; language disabled")
                }
            }
        }

        let (sandbox, refusal) = if cfg!(target_os = "linux") || config.allow_unsandboxed {
            if !cfg!(target_os = "linux") {
                tracing::warn!(
                    "RUNNER_ALLOW_UNSANDBOXED=1: jobs run WITHOUT any sandbox (development only)"
                );
            }
            (Some(Sandbox::new()?), String::new())
        } else {
            (
                None,
                format!(
                    "the sandbox needs Linux and this runner is on {}; set RUNNER_ALLOW_UNSANDBOXED=1 \
                     to run jobs unconfined for local development",
                    std::env::consts::OS
                ),
            )
        };

        prepare_work_root(&config.work_dir)?;
        if languages.contains_key(&Language::Go) && !config.go_warm_cache.is_dir() {
            tracing::warn!(
                path = %config.go_warm_cache.display(),
                "no warm Go build cache: every Go job recompiles the standard library"
            );
        }

        Ok(Self {
            pool: SlotPool::new(capacity, root),
            config,
            languages,
            sandbox,
            refusal,
            lambda,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Take an execution slot, or `None` when the runner is full.
    pub fn try_acquire(&self) -> Option<SlotGuard> {
        self.pool.try_acquire()
    }

    pub fn health(&self) -> RunnerHealth {
        RunnerHealth {
            status: if self.sandbox.is_some() {
                "ok"
            } else {
                "refusing"
            }
            .to_string(),
            version: RUNNER_VERSION.to_string(),
            protocol: PROTOCOL_VERSION,
            languages: self
                .languages
                .iter()
                .map(|(lang, version)| LanguageInfo {
                    id: lang.id().to_string(),
                    version: version.clone(),
                })
                .collect(),
            capacity: self.pool.capacity(),
            in_flight: self.pool.in_flight(),
        }
    }

    /// Run one job. Never fails: every problem becomes a response, either a
    /// case status (the program's fault) or a `RunnerError` (not the program's
    /// fault).
    pub async fn execute(&self, req: ExecuteRequest, slot: &SlotGuard) -> ExecuteResponse {
        let started = Instant::now();
        let job = log_id(&req.job_id);
        tracing::info!(
            job = %job,
            language = %log_id(&req.language),
            source_bytes = req.source.len(),
            cases = req.cases.len(),
            stdin_bytes = req.cases.iter().map(|c| c.stdin.len()).sum::<usize>(),
            slot = slot.index(),
            "job started"
        );

        let mut resp = match self.admit(&req) {
            Ok((lang, sandbox)) => self.run_job(&req, lang, sandbox, slot, started).await,
            Err(error) => ExecuteResponse::refused(&req.job_id, &req.language, error),
        };
        resp.runner_version = RUNNER_VERSION.to_string();
        resp.duration_ms = millis(started.elapsed());

        let count = |s: CaseStatus| resp.cases.iter().filter(|c| c.status == s).count();
        tracing::info!(
            job = %job,
            error = ?resp.error.as_ref().map(|e| e.code),
            compiled = resp.compiled(),
            ok = count(CaseStatus::Ok),
            runtime_error = count(CaseStatus::RuntimeError),
            timeout = count(CaseStatus::Timeout),
            memory_limit = count(CaseStatus::MemoryLimit),
            output_limit = count(CaseStatus::OutputLimit),
            skipped = count(CaseStatus::Skipped),
            duration_ms = resp.duration_ms,
            "job finished"
        );
        resp
    }

    /// Everything that can refuse a job before any file is written.
    fn admit(&self, req: &ExecuteRequest) -> Result<(Language, &Sandbox), RunnerError> {
        req.validate()
            .map_err(|m| RunnerError::new(RunnerErrorCode::BadRequest, m))?;
        let lang = match Language::from_id(&req.language) {
            Some(lang) if self.languages.contains_key(&lang) => lang,
            Some(lang) => {
                return Err(RunnerError::new(
                    RunnerErrorCode::UnsupportedLanguage,
                    format!(
                        "the {} toolchain is not installed on this runner",
                        lang.id()
                    ),
                ))
            }
            None => {
                return Err(RunnerError::new(
                    RunnerErrorCode::UnsupportedLanguage,
                    "unknown language id",
                ))
            }
        };
        let sandbox = self
            .sandbox
            .as_ref()
            .ok_or_else(|| RunnerError::new(RunnerErrorCode::Internal, self.refusal.clone()))?;
        Ok((lang, sandbox))
    }

    async fn run_job(
        &self,
        req: &ExecuteRequest,
        lang: Language,
        sandbox: &Sandbox,
        slot: &SlotGuard,
        started: Instant,
    ) -> ExecuteResponse {
        let limits = self.config.ceilings.clamp(&req.limits);
        let deadline = started + limits.total_timeout;
        let creds = slot.credentials();
        let scope = match creds {
            Some(c) => ReapScope::Uid(c.uid),
            None if self.lambda => ReapScope::AllButAncestors,
            None => ReapScope::Descendants,
        };
        let job_dir = self
            .config
            .work_dir
            .join(format!("job-{}", uuid::Uuid::new_v4().simple()));
        // Armed before the directory exists, so every exit path below —
        // including a panic or the request future being dropped — tears down.
        let _teardown = Teardown {
            dir: job_dir.clone(),
            scope,
            work_root: &self.config.work_dir,
        };
        let layout = Layout::new(job_dir, lang);
        let job = Job {
            lang,
            layout,
            limits,
            creds,
            scope,
            sandbox,
        };
        let mut resp = ExecuteResponse {
            protocol: PROTOCOL_VERSION,
            job_id: req.job_id.clone(),
            language: req.language.clone(),
            compile: None,
            cases: Vec::with_capacity(req.cases.len()),
            error: None,
            runner_version: String::new(),
            duration_ms: 0,
        };

        if let Err(e) = job.prepare(&req.source, &self.config.go_warm_cache) {
            tracing::error!(error = %e, "could not prepare the job directory");
            resp.error = Some(RunnerError::new(
                RunnerErrorCode::Internal,
                "could not prepare the job directory",
            ));
            return resp;
        }

        if let Some(argv) = lang.compile_argv(&job.layout) {
            let wall = limits.compile_timeout.min(remaining(deadline));
            match job.compile(argv, wall).await {
                Ok(outcome) => {
                    let ok = outcome.ok;
                    resp.compile = Some(outcome);
                    if !ok {
                        return resp;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "could not start the compiler");
                    resp.error = Some(RunnerError::new(
                        RunnerErrorCode::Internal,
                        "could not start the compiler",
                    ));
                    return resp;
                }
            }
        }
        if let Err(e) = job.seal() {
            tracing::error!(error = %e, "could not seal the build outputs");
            resp.error = Some(RunnerError::new(
                RunnerErrorCode::Internal,
                "could not prepare the build outputs",
            ));
            return resp;
        }

        for (index, case) in req.cases.iter().enumerate() {
            let slice = remaining(deadline);
            if slice < MIN_CASE_SLICE {
                resp.cases.push(skipped(&case.id));
                continue;
            }
            match job
                .run_case(index, case, limits.run_timeout.min(slice))
                .await
            {
                Ok(outcome) => resp.cases.push(outcome),
                Err(e) => {
                    tracing::error!(error = %e, case = index, "could not run a case");
                    resp.cases.clear();
                    resp.error = Some(RunnerError::new(
                        RunnerErrorCode::Internal,
                        "could not start the program",
                    ));
                    return resp;
                }
            }
        }
        resp
    }
}

/// One job's fixed context.
struct Job<'a> {
    lang: Language,
    layout: Layout,
    limits: EffectiveLimits,
    creds: Option<Credentials>,
    scope: ReapScope,
    sandbox: &'a Sandbox,
}

impl Job<'_> {
    /// Create the directory tree (see the module docs) and write the source.
    fn prepare(&self, source: &str, warm_cache: &Path) -> io::Result<()> {
        let l = &self.layout;
        let c = self.creds;
        make_dir(&l.job_dir, Access::ReadOnly, c)?;
        make_dir(&l.scratch, Access::Writable, c)?;
        make_dir(&l.cases_dir, Access::ReadOnly, c)?;
        match self.lang {
            Language::Go => {
                make_dir(&l.bin_dir, Access::Writable, c)?;
                make_dir(&l.go_path, Access::Writable, c)?;
                make_dir(&l.go_cache, Access::Writable, c)?;
                link_farm(warm_cache, &l.go_cache, c)?;
                mark_cache_trimmed(&l.go_cache, c)?;
            }
            Language::Cpp => make_dir(&l.bin_dir, Access::Writable, c)?,
            Language::Java => make_dir(&l.classes_dir, Access::Writable, c)?,
            Language::Python => {}
        }
        write_file(&l.source, source.as_bytes(), c)
    }

    fn spec(&self, argv: Vec<String>, cwd: PathBuf, env: Vec<(String, String)>) -> ExecSpec {
        ExecSpec {
            argv,
            cwd,
            env,
            stdin: Vec::new(),
            wall: Duration::ZERO,
            memory: self.lang.run_memory(self.limits.memory_mb),
            max_output_bytes: self.limits.max_output_bytes,
            credentials: self.creds,
            processes: exec::process_limit(self.creds),
            reap: self.scope,
        }
    }

    async fn compile(&self, argv: Vec<String>, wall: Duration) -> io::Result<CompileOutcome> {
        let l = &self.layout;
        let mut env = base_env(&l.scratch);
        env.extend(self.lang.compile_env(l));
        let spec = ExecSpec {
            wall,
            memory: self.lang.compile_memory(self.limits.compile_memory_mb),
            ..self.spec(argv, l.job_dir.clone(), env)
        };
        let res = exec::run(spec, self.sandbox).await?;
        let (mut text, truncated) =
            output::combine_capped(&res.stdout, &res.stderr, self.limits.max_output_bytes);
        if let Some(note) = compile_note(&res, &self.limits) {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&note);
        }
        Ok(CompileOutcome {
            ok: res.termination == Termination::Exited && res.exit_code == Some(0),
            output: text,
            timed_out: res.termination == Termination::TimedOut,
            truncated,
            duration_ms: millis(res.duration),
        })
    }

    /// After a successful build: drop the compile-only state and make the
    /// build outputs read-only to the job, so cases cannot pass files to each
    /// other through them (or corrupt the program for later cases).
    fn seal(&self) -> io::Result<()> {
        let l = &self.layout;
        for scratch in [&l.scratch, &l.go_cache, &l.go_path] {
            remove_tree(scratch)?;
        }
        // Only a root runner can take ownership away from the job; without
        // root the job *is* the owner and could simply undo it.
        #[cfg(unix)]
        if let Some(creds) = self.creds {
            for built in [&l.bin_dir, &l.classes_dir] {
                seal_tree(built, creds)?;
            }
        }
        Ok(())
    }

    async fn run_case(
        &self,
        index: usize,
        case: &CaseInput,
        wall: Duration,
    ) -> io::Result<CaseOutcome> {
        let l = &self.layout;
        let dir = l.cases_dir.join(index.to_string());
        make_dir(&dir, Access::Writable, self.creds)?;
        let mut env = base_env(&dir);
        env.extend(self.lang.run_env(l, self.limits.memory_mb));
        let argv = self.lang.run_argv(l, self.limits.memory_mb, &dir);
        let spec = ExecSpec {
            stdin: case.stdin.as_bytes().to_vec(),
            wall,
            ..self.spec(argv, dir.clone(), env)
        };
        let result = exec::run(spec, self.sandbox).await;
        // Between cases, not just between jobs: whatever this case started or
        // wrote outside its directory is gone before the next one begins.
        exec::reap(self.scope);
        let removed = remove_tree(&dir);
        exec::sweep_shared_tmp(self.scope, &l.job_dir);
        let res = result?;
        removed?;

        let stderr = output::decode(&res.stderr);
        Ok(CaseOutcome {
            id: case.id.clone(),
            status: classify(res.termination, res.exit_code, res.signal, &stderr),
            stdout: output::decode(&res.stdout),
            stderr,
            exit_code: res.exit_code,
            signal: res.signal,
            duration_ms: millis(res.duration),
            stdout_truncated: res.stdout_truncated,
            stderr_truncated: res.stderr_truncated,
        })
    }
}

/// Removes the job directory and everything the job started, however the job
/// ended. Runs synchronously in `Drop`: a few syscalls and a small tree.
struct Teardown<'a> {
    dir: PathBuf,
    scope: ReapScope,
    work_root: &'a Path,
}

impl Drop for Teardown<'_> {
    fn drop(&mut self) {
        exec::reap(self.scope);
        if matches!(self.scope, ReapScope::Uid(_)) {
            // The work root is root's; the job could only write inside its
            // own directory.
            if let Err(e) = remove_tree(&self.dir) {
                tracing::error!(error = %e, "could not remove a job directory");
            }
        } else if let Err(e) = reset_work_root(self.work_root) {
            tracing::error!(error = %e, "could not reset the work root");
        }
        exec::sweep_shared_tmp(self.scope, self.work_root);
        if self.dir.exists() {
            tracing::error!(path = %self.dir.display(), "a job directory survived teardown");
        }
    }
}

/// Replace the work root with a fresh, empty directory, job directory and
/// all. Sharing the runner's uid, a job owns the root too: it can drop files
/// there, change its permissions, or swap the directory for a symlink that
/// later jobs' directories would otherwise be created through. Jobs run one
/// at a time in that mode, so whatever is at the path now is this job's
/// doing, and none of it is kept or followed.
fn reset_work_root(root: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(root).is_ok_and(|m| m.is_dir()) {
        remove_tree(root)?;
    }
    prepare_work_root(root)
}

/// The environment every child starts from (before language additions).
/// Nothing is inherited from the runner.
fn base_env(home: &Path) -> Vec<(String, String)> {
    let home = home.to_string_lossy().into_owned();
    vec![
        ("PATH".into(), child_path()),
        ("HOME".into(), home.clone()),
        ("TMPDIR".into(), home),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
    ]
}

/// Explain a failed build that the runner caused, so the student does not
/// stare at an empty compiler log.
fn compile_note(res: &ExecResult, limits: &EffectiveLimits) -> Option<String> {
    match res.termination {
        Termination::TimedOut => Some(format!(
            "[runner] compilation did not finish within {} ms",
            limits.compile_timeout.as_millis()
        )),
        Termination::MemoryExceeded => Some(format!(
            "[runner] the compiler exceeded its {} MiB memory limit",
            limits.compile_memory_mb
        )),
        Termination::OutputExceeded => Some(format!(
            "[runner] compiler output exceeded {} bytes",
            limits.max_output_bytes
        )),
        Termination::Exited => None,
    }
}

fn skipped(id: &str) -> CaseOutcome {
    CaseOutcome {
        id: id.to_string(),
        status: CaseStatus::Skipped,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        signal: None,
        duration_ms: 0,
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

fn remaining(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// The caller's ids are echoed back verbatim but logged defanged: bounded
/// length, no control characters (a newline could forge a log line in the
/// human-readable format).
fn log_id(id: &str) -> String {
    id.chars()
        .take(64)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

fn probe_languages() -> BTreeMap<Language, String> {
    // Probes are independent and the JVM ones take a few hundred ms each.
    std::thread::scope(|s| {
        let probes: Vec<_> = Language::ALL
            .into_iter()
            .map(|lang| (lang, s.spawn(move || lang.probe())))
            .collect();
        probes
            .into_iter()
            .filter_map(|(lang, h)| Some((lang, h.join().ok()??)))
            .collect()
    })
}

/// Create the work root and clear out the job directories a previous, crashed
/// runner left behind (its jobs are long dead; their files must not linger).
/// Only `job-*` entries are touched, so even a misconfigured root never loses
/// anything the runner did not create.
fn prepare_work_root(root: &Path) -> io::Result<()> {
    // Anything at the path that is not a real directory (a symlink or file a
    // same-uid job left there) is removed, never followed.
    if std::fs::symlink_metadata(root).is_ok_and(|m| !m.is_dir()) {
        std::fs::remove_file(root)?;
    }
    std::fs::create_dir_all(root)?;
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with("job-") {
            remove_tree(&entry.path())?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Traversable (jobs must reach their own directory), not listable, not
        // writable: a job can neither discover nor create siblings.
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o711))?;
    }
    Ok(())
}

/// Who may write a path. `ReadOnly` paths are owned by the runner and readable
/// by the job through its group.
#[derive(Clone, Copy, Debug)]
enum Access {
    Writable,
    ReadOnly,
}

fn make_dir(path: &Path, access: Access, creds: Option<Credentials>) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(match access {
                Access::Writable => 0o700,
                Access::ReadOnly => 0o750,
            })
            .create(path)?;
    }
    #[cfg(not(unix))]
    std::fs::DirBuilder::new().create(path)?;
    set_owner(path, access, creds)
}

/// Write a new file (never through an existing path: `create_new`).
fn write_file(path: &Path, contents: &[u8], creds: Option<Credentials>) -> io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o640);
    }
    options.open(path)?.write_all(contents)?;
    set_owner(path, Access::ReadOnly, creds)
}

fn set_owner(path: &Path, access: Access, creds: Option<Credentials>) -> io::Result<()> {
    #[cfg(unix)]
    if let Some(c) = creds {
        let uid = match access {
            Access::Writable => c.uid,
            // Credentials only exist when the runner is root.
            Access::ReadOnly => 0,
        };
        std::os::unix::fs::lchown(path, Some(uid), Some(c.gid))?;
    }
    #[cfg(not(unix))]
    let _ = (path, access, creds);
    Ok(())
}

/// Hand a build output tree to the runner, readable (and, for programs,
/// executable) by the job's group, writable by nobody but root.
///
/// Only what a build really produces is re-owned: directories and
/// single-link regular files, owned by the job's uid. Anything else was
/// planted by the compiler (i.e. the job) and is removed instead — a symlink,
/// a FIFO, or a hard link to a file the job cannot read (`/etc/shadow`, say),
/// which the chown below would otherwise hand to the job's group. The compile
/// step's processes are all dead by now (the executor reaps the job's uid
/// after every run), so nothing can swap an entry between check and change.
#[cfg(unix)]
fn seal_tree(path: &Path, creds: Credentials) -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let kind = meta.file_type();
    let built = meta.uid() == creds.uid && (kind.is_dir() || (kind.is_file() && meta.nlink() == 1));
    if !built {
        return if kind.is_dir() {
            remove_tree(path)
        } else {
            std::fs::remove_file(path)
        };
    }
    if meta.is_dir() {
        for entry in std::fs::read_dir(path)? {
            seal_tree(&entry?.path(), creds)?;
        }
    }
    set_owner(path, Access::ReadOnly, Some(creds))?;
    let exec_bits = if meta.is_dir() || meta.permissions().mode() & 0o100 != 0 {
        0o110
    } else {
        0
    };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o640 | exec_bits))
}

/// `remove_dir_all` that tolerates a missing path and, when a job made its own
/// files unreadable (same-uid mode, where the runner is not root), restores
/// owner permissions and retries.
fn remove_tree(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(first) => {
            #[cfg(unix)]
            {
                restore_owner_access(path);
                match std::fs::remove_dir_all(path) {
                    Ok(()) => return Ok(()),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
                    Err(_) => {}
                }
            }
            Err(first)
        }
    }
}

#[cfg(unix)]
fn restore_owner_access(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !meta.is_dir() {
        return;
    }
    let mode = meta.permissions().mode() | 0o700;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            restore_owner_access(&entry.path());
        }
    }
}

/// Populate this job's `GOCACHE` with links to the image's pre-built one.
///
/// The warm cache holds the compiled standard library, so a job only compiles
/// its own package (~0.3 s instead of ~10 s). It is never *shared writable*:
/// Go trusts a cache entry by name and size, so a job that could write to a
/// shared cache could plant a trojaned `fmt` for the next user. Instead each
/// job gets its own directory tree whose files are hard links (same
/// filesystem, runner is root) or symlinks (otherwise, e.g. Lambda's `/tmp`)
/// to the warm files, which are root-owned and 0444: the job reads them and
/// adds its own entries beside them, but cannot modify a shared inode — and
/// Go never rewrites an existing entry, it only adds new ones or replaces a
/// *name* (which changes only this job's view).
pub fn link_farm(warm: &Path, dst: &Path, creds: Option<Credentials>) -> io::Result<()> {
    let entries = match std::fs::read_dir(warm) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if name == TRIM_FILE {
            continue; // replaced by a fresh, job-owned one
        }
        let (from, to) = (entry.path(), dst.join(&name));
        let kind = entry.file_type()?;
        if kind.is_dir() {
            make_dir(&to, Access::Writable, creds)?;
            link_farm(&from, &to, creds)?;
        } else if kind.is_file() {
            link_file(&from, &to)?;
        }
    }
    Ok(())
}

fn link_file(from: &Path, to: &Path) -> io::Result<()> {
    if std::fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(from, to)
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(from, to).map(drop)
    }
}

/// Go's cache records its last trim in `trim.txt` (Unix seconds) and walks
/// the whole cache if that is more than a day old. The warm cache's own record
/// dates from the image build, so every job would pay for a trim of links it
/// is about to throw away; a fresh record skips it. Should the format ever
/// change, Go just trims — correct, only slower.
const TRIM_FILE: &str = "trim.txt";

fn mark_cache_trimmed(cache: &Path, creds: Option<Credentials>) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = cache.join(TRIM_FILE);
    write_file(&path, now.to_string().as_bytes(), creds)?;
    set_owner(&path, Access::Writable, creds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logged_ids_cannot_forge_log_lines() {
        assert_eq!(log_id("run-42"), "run-42");
        assert_eq!(log_id("a\nb\r\x1b[31m"), "a?b??[31m");
        assert_eq!(log_id(&"x".repeat(1000)).len(), 64);
    }

    #[test]
    fn children_start_from_a_minimal_environment() {
        let env = base_env(Path::new("/w/job/cases/0"));
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL"]);
        let get = |k: &str| {
            env.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("HOME"), Some("/w/job/cases/0"));
        assert_eq!(
            get("TMPDIR"),
            get("HOME"),
            "each case keeps its temp files at home"
        );
        assert_eq!(get("LANG"), Some("C.UTF-8"));
        if cfg!(target_os = "linux") {
            assert!(
                !get("PATH").unwrap().contains('.'),
                "no relative PATH entries"
            );
        }
    }

    #[test]
    fn skipped_cases_carry_their_id_and_nothing_else() {
        let s = skipped("c7");
        assert_eq!(s.id, "c7");
        assert_eq!(s.status, CaseStatus::Skipped);
        assert!(s.stdout.is_empty() && s.exit_code.is_none());
    }

    #[test]
    fn the_go_cache_farm_mirrors_the_warm_cache_without_copying_its_trim_record() {
        let warm = tempfile::tempdir().unwrap();
        std::fs::create_dir(warm.path().join("ab")).unwrap();
        std::fs::write(warm.path().join("ab").join("abcd-d"), b"object").unwrap();
        std::fs::write(warm.path().join("README"), b"cache").unwrap();
        std::fs::write(warm.path().join(TRIM_FILE), b"1").unwrap();

        let job = tempfile::tempdir().unwrap();
        let cache = job.path().join("gocache");
        make_dir(&cache, Access::Writable, None).unwrap();
        link_farm(warm.path(), &cache, None).unwrap();
        mark_cache_trimmed(&cache, None).unwrap();

        assert_eq!(
            std::fs::read(cache.join("ab").join("abcd-d")).unwrap(),
            b"object"
        );
        assert_eq!(std::fs::read(cache.join("README")).unwrap(), b"cache");
        let trimmed: u64 = std::fs::read_to_string(cache.join(TRIM_FILE))
            .unwrap()
            .parse()
            .unwrap();
        assert!(trimmed > 1, "a fresh trim record, not the warm cache's");
        // New entries land in the job's own directory, not the warm one.
        std::fs::write(cache.join("ab").join("new-d"), b"mine").unwrap();
        assert!(!warm.path().join("ab").join("new-d").exists());
    }

    #[test]
    fn a_missing_warm_cache_just_means_a_cold_build() {
        let job = tempfile::tempdir().unwrap();
        link_farm(&job.path().join("absent"), job.path(), None).unwrap();
    }

    #[test]
    fn the_work_root_is_emptied_on_startup() {
        let root = tempfile::tempdir().unwrap();
        let stale = root.path().join("job-stale");
        std::fs::create_dir_all(stale.join("cases").join("0")).unwrap();
        std::fs::write(stale.join("main.go"), b"x").unwrap();
        std::fs::write(root.path().join("not-ours"), b"keep").unwrap();
        prepare_work_root(root.path()).unwrap();
        assert!(!stale.exists());
        assert!(
            root.path().join("not-ours").exists(),
            "only job directories are removed"
        );
    }

    #[test]
    fn resetting_the_work_root_leaves_it_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("work");
        std::fs::create_dir_all(root.join("job-x").join("cases")).unwrap();
        std::fs::write(root.join("dropped"), b"x").unwrap();
        reset_work_root(&root).unwrap();
        assert!(root.is_dir());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_work_root_swapped_for_a_symlink_is_replaced_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("keep"), b"not the runner's").unwrap();
        let root = dir.path().join("work");
        std::os::unix::fs::symlink(&elsewhere, &root).unwrap();

        reset_work_root(&root).unwrap();

        let meta = std::fs::symlink_metadata(&root).unwrap();
        assert!(meta.is_dir(), "a real directory again, not the link");
        assert!(
            elsewhere.join("keep").exists(),
            "the link's target is untouched"
        );
    }

    #[test]
    fn remove_tree_tolerates_what_is_already_gone() {
        let dir = tempfile::tempdir().unwrap();
        remove_tree(&dir.path().join("never-existed")).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sealing_hands_over_only_what_the_build_made() {
        use std::os::unix::fs::{lchown, symlink, MetadataExt, PermissionsExt};
        if !crate::hardening::is_root() {
            eprintln!("SKIP: sealing changes ownership, which needs root");
            return;
        }
        let job = Credentials {
            uid: 29_999,
            gid: 29_999,
        };
        let root = tempfile::tempdir().unwrap();
        let mode =
            |p: &Path, m: u32| std::fs::set_permissions(p, std::fs::Permissions::from_mode(m));
        let secret = root.path().join("secret");
        std::fs::write(&secret, b"root only").unwrap();
        mode(&secret, 0o600).unwrap();

        let bin = root.path().join("bin");
        make_dir(&bin, Access::Writable, Some(job)).unwrap();
        let prog = bin.join("prog");
        std::fs::write(&prog, b"\x7fELF").unwrap();
        mode(&prog, 0o755).unwrap();
        lchown(&prog, Some(job.uid), Some(job.gid)).unwrap();
        // What a compromised compiler could leave behind: links to a file
        // the job cannot read, and one of its own files linked from outside.
        std::fs::hard_link(&secret, bin.join("hard")).unwrap();
        symlink(&secret, bin.join("soft")).unwrap();
        let twice = bin.join("twice");
        std::fs::write(&twice, b"x").unwrap();
        lchown(&twice, Some(job.uid), Some(job.gid)).unwrap();
        std::fs::hard_link(&twice, root.path().join("outside")).unwrap();

        seal_tree(&bin, job).unwrap();

        let mut left: Vec<String> = std::fs::read_dir(&bin)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["prog"], "only the build's own output survives");
        let p = std::fs::metadata(&prog).unwrap();
        assert_eq!((p.uid(), p.gid(), p.mode() & 0o777), (0, job.gid, 0o750));
        let s = std::fs::metadata(&secret).unwrap();
        assert_eq!(
            (s.uid(), s.gid(), s.mode() & 0o777),
            (0, 0, 0o600),
            "a planted link's target is never re-owned"
        );
    }
}
