//! The languages this runner executes, baked into the binary.
//!
//! A request names a language *id*; it never supplies a command line, a flag
//! or an environment variable. Everything a toolchain is invoked with is
//! decided here, which is what makes "a compromised API cannot make the runner
//! execute an arbitrary argv" true.
//!
//! Each language also owns its **memory policy**, because the one tool that
//! would work for all of them — a cgroup — is not available to an unprivileged
//! container. The rlimit that is available, `RLIMIT_AS`, caps *virtual* address
//! space, which is a good proxy for memory use in C++ and CPython but useless
//! for Go and the JVM: both reserve address space far beyond what they touch
//! (Go's heap arenas, the JVM's 1 GiB compressed class space and code cache),
//! so an address-space cap small enough to mean anything stops them from
//! starting at all. Those two get their runtime's own heap cap (`GOMEMLIMIT`,
//! `-Xmx`) instead, and every language is backed by the executor's RSS
//! watchdog, which measures what is actually resident.

use crate::exec::MemoryLimit;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// `PATH` for every process the runner starts, toolchain probes included, so
/// a probe can never find a binary the jobs themselves would not.
///
/// On Linux this is fixed: it names the image's toolchain directories and
/// nothing a job could write to. Elsewhere (unsandboxed development only) the
/// developer's own `PATH` is used, since toolchains live wherever they were
/// installed.
pub fn child_path() -> String {
    if cfg!(target_os = "linux") {
        "/usr/local/go/bin:/usr/local/bin:/usr/bin:/bin".to_string()
    } else {
        std::env::var("PATH").unwrap_or_default()
    }
}

/// What the JVM needs besides its heap: metaspace, the (C1-only) code cache,
/// GC structures, thread stacks and `libjvm` itself. Measured at 50–70 MiB
/// resident for a problem-sized program with the flags below; 96 leaves room.
const JVM_OVERHEAD_MB: u64 = 96;
/// Below this the JVM refuses to start, which would be a confusing error for
/// a request that merely asked for very little memory.
const JVM_MIN_HEAP_MB: u64 = 16;

const MIB: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    Go,
    Cpp,
    Java,
    Python,
}

/// Where one job's files live. Built by the engine per job; the language
/// methods below only ever point commands at these paths.
#[derive(Clone, Debug)]
pub struct Layout {
    pub job_dir: PathBuf,
    /// The submitted program.
    pub source: PathBuf,
    /// Compiler output directory (C++, Go).
    pub bin_dir: PathBuf,
    /// The compiled executable (C++, Go).
    pub program: PathBuf,
    /// `javac` output (Java).
    pub classes_dir: PathBuf,
    /// This job's private, writable Go build cache (see `engine::link_farm`).
    pub go_cache: PathBuf,
    pub go_path: PathBuf,
    /// `$HOME` and `$TMPDIR` of the compile step. Removed once the build is
    /// done: each case gets its own directory for both instead, so no case
    /// can leave anything for the next one to find.
    pub scratch: PathBuf,
    /// Parent of the per-case working directories.
    pub cases_dir: PathBuf,
}

impl Layout {
    pub fn new(job_dir: PathBuf, language: Language) -> Self {
        let bin_dir = job_dir.join("bin");
        Self {
            source: job_dir.join(language.source_file()),
            program: bin_dir.join("prog"),
            bin_dir,
            classes_dir: job_dir.join("classes"),
            go_cache: job_dir.join("gocache"),
            go_path: job_dir.join("gopath"),
            scratch: job_dir.join("tmp"),
            cases_dir: job_dir.join("cases"),
            job_dir,
        }
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn owned(argv: &[&str]) -> Vec<String> {
    argv.iter().map(|a| a.to_string()).collect()
}

impl Language {
    pub const ALL: [Language; 4] = [
        Language::Go,
        Language::Cpp,
        Language::Java,
        Language::Python,
    ];

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            Language::Go => "go",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::Python => "python",
        }
    }

    /// The file the source is written to. Java insists it be named after the
    /// public class, which the problem harnesses call `Main`.
    pub fn source_file(self) -> &'static str {
        match self {
            Language::Go => "main.go",
            Language::Cpp => "main.cpp",
            Language::Java => "Main.java",
            Language::Python => "main.py",
        }
    }

    /// Commands that must all succeed for the language to be offered; the
    /// first one's first output line is the version reported in `/healthz`.
    /// Java needs both halves of the JDK, so a JRE-only image does not
    /// advertise a language it cannot compile.
    fn probe_commands(self) -> &'static [&'static [&'static str]] {
        match self {
            Language::Go => &[&["go", "version"]],
            Language::Cpp => &[&["g++", "--version"]],
            Language::Java => &[&["javac", "-version"], &["java", "-version"]],
            Language::Python => &[&["python3", "--version"]],
        }
    }

    /// Probe for the toolchain; `None` when it is missing or broken, which
    /// makes the language `unsupported_language` rather than a crash.
    pub fn probe(self) -> Option<String> {
        let mut version = None;
        for argv in self.probe_commands() {
            let line = run_probe(argv)?;
            version.get_or_insert(line);
        }
        version
    }

    /// The compile command, or `None` for Python.
    ///
    /// Python has no separate compile step: a syntax error surfaces as a
    /// runtime error on every case with the interpreter's own message, which
    /// is exactly what the student would see locally, and the protocol reports
    /// `compile: None` for it. A `py_compile` pass would cost a second
    /// interpreter start per job to say the same thing earlier.
    pub fn compile_argv(self, l: &Layout) -> Option<Vec<String>> {
        match self {
            Language::Go => Some(vec![
                "go".into(),
                "build".into(),
                "-o".into(),
                s(&l.program),
                s(&l.source),
            ]),
            Language::Cpp => Some(vec![
                "g++".into(),
                "-std=c++17".into(),
                "-O2".into(),
                "-pipe".into(),
                "-o".into(),
                s(&l.program),
                s(&l.source),
            ]),
            // `javac` is a JVM too: the same fast-start flags as the run step
            // cut its startup roughly in half, and `-UsePerfData` stops it
            // writing `/tmp/hsperfdata_<uid>` outside the job directory.
            Language::Java => {
                let mut argv = owned(&[
                    "javac",
                    "-J-XX:+UseSerialGC",
                    "-J-XX:TieredStopAtLevel=1",
                    "-J-Xshare:auto",
                    "-J-XX:-UsePerfData",
                    "-nowarn",
                    "-encoding",
                    "UTF-8",
                ]);
                argv.push(format!("-J-Djava.io.tmpdir={}", s(&l.scratch)));
                argv.push("-d".into());
                argv.push(s(&l.classes_dir));
                argv.push(s(&l.source));
                Some(argv)
            }
            Language::Python => None,
        }
    }

    /// The run command for one case. `tmp` is that case's private directory.
    pub fn run_argv(self, l: &Layout, memory_mb: u64, tmp: &Path) -> Vec<String> {
        match self {
            Language::Go | Language::Cpp => vec![s(&l.program)],
            Language::Java => {
                let heap = java_heap_mb(memory_mb);
                vec![
                    "java".into(),
                    format!("-Xmx{heap}m"),
                    // Small initial heap: no point touching memory up front.
                    format!("-Xms{}m", heap.min(16)),
                    // Deep recursion is normal in DSA solutions.
                    "-Xss64m".into(),
                    // One GC thread instead of one per core: fewer threads to
                    // count against RLIMIT_NPROC and less memory overhead.
                    "-XX:+UseSerialGC".into(),
                    // C1 only: problem-sized programs finish before C2 would pay off.
                    "-XX:TieredStopAtLevel=1".into(),
                    "-Xshare:auto".into(),
                    // No `/tmp/hsperfdata_<uid>` outside the job directory.
                    "-XX:-UsePerfData".into(),
                    format!("-Djava.io.tmpdir={}", s(tmp)),
                    "-cp".into(),
                    s(&l.classes_dir),
                    "Main".into(),
                ]
            }
            // -I: isolated mode (ignores PYTHON* variables and the user site
            // directory, and keeps the script's directory off sys.path).
            // -B: no .pyc files.
            Language::Python => vec!["python3".into(), "-I".into(), "-B".into(), s(&l.source)],
        }
    }

    /// Language-specific environment for the compile step, added on top of
    /// the engine's minimal base environment.
    pub fn compile_env(self, l: &Layout) -> Vec<(String, String)> {
        match self {
            Language::Go => go_env(l),
            Language::Cpp | Language::Java | Language::Python => Vec::new(),
        }
    }

    /// Language-specific environment for each case.
    pub fn run_env(self, l: &Layout, memory_mb: u64) -> Vec<(String, String)> {
        match self {
            Language::Go => {
                let mut env = go_env(l);
                // The GC starts working hard at 90% of the limit, so a program
                // near its budget slows down instead of being killed by the
                // RSS watchdog at the first spike.
                let soft = memory_mb * MIB / 10 * 9;
                env.push(("GOMEMLIMIT".into(), soft.to_string()));
                env
            }
            Language::Cpp | Language::Java | Language::Python => Vec::new(),
        }
    }

    /// Memory policy for the compile step (`compile_mb` is already clamped).
    pub fn compile_memory(self, compile_mb: u64) -> MemoryLimit {
        let bytes = compile_mb * MIB;
        match self {
            // cc1plus is plain C++ and allocates what it uses, so its address
            // space is a fair measure; the generous ceiling covers
            // `<bits/stdc++.h>` at -O2.
            Language::Cpp => MemoryLimit {
                address_space: Some(bytes),
                resident: bytes,
            },
            Language::Go | Language::Java | Language::Python => MemoryLimit {
                address_space: None,
                resident: bytes,
            },
        }
    }

    /// Memory policy for each case (`memory_mb` is already clamped).
    pub fn run_memory(self, memory_mb: u64) -> MemoryLimit {
        let bytes = memory_mb * MIB;
        match self {
            // A failed allocation surfaces as `std::bad_alloc` / `MemoryError`,
            // which the classifier recognises: the student sees *why*.
            Language::Cpp | Language::Python => MemoryLimit {
                address_space: Some(bytes),
                resident: bytes,
            },
            Language::Go | Language::Java => MemoryLimit {
                address_space: None,
                resident: bytes,
            },
        }
    }
}

/// `-Xmx` for a total budget: whatever is left after the JVM's own overhead.
pub fn java_heap_mb(memory_mb: u64) -> u64 {
    memory_mb
        .saturating_sub(JVM_OVERHEAD_MB)
        .max(JVM_MIN_HEAP_MB)
}

/// The Go environment shared by build and run. Every variable here closes a
/// door: no toolchain download (`GOTOOLCHAIN=local`), no module fetching or
/// `go.mod` needed for a single std-only file (`GO111MODULE=off`,
/// `GOPROXY=off`), no user config file (`GOENV=off`), no workspace lookup
/// (`GOWORK=off`), no C toolchain (`CGO_ENABLED=0`, which also makes the
/// binary static), and caches that live inside the job.
fn go_env(l: &Layout) -> Vec<(String, String)> {
    [
        ("GOTOOLCHAIN", "local".to_string()),
        ("GO111MODULE", "off".to_string()),
        ("GOPROXY", "off".to_string()),
        ("GOENV", "off".to_string()),
        ("GOWORK", "off".to_string()),
        ("CGO_ENABLED", "0".to_string()),
        ("GOCACHE", s(&l.go_cache)),
        ("GOPATH", s(&l.go_path)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// Run one trusted probe command and return the first non-empty output line.
///
/// Probes run once at startup, unsandboxed, on the image's own toolchains —
/// never on user input. A hung probe is abandoned after a few seconds rather
/// than blocking startup forever; its thread is left to finish on its own.
fn run_probe(argv: &[&str]) -> Option<String> {
    let (program, args) = argv.split_first()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env("PATH", child_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(cmd.output());
    });
    let out = rx.recv_timeout(Duration::from_secs(10)).ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    // Some tools print their version on stderr (`java -version`), some on
    // stdout; take whichever speaks first.
    [out.stdout, out.stderr].iter().find_map(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_string)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(lang: Language) -> Layout {
        Layout::new(PathBuf::from("/w/job-1"), lang)
    }

    #[test]
    fn ids_round_trip_and_nothing_else_resolves() {
        for lang in Language::ALL {
            assert_eq!(Language::from_id(lang.id()), Some(lang));
        }
        for bad in ["", "Go", "c++", "rust", "go ", "../go", "python3"] {
            assert_eq!(Language::from_id(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn java_source_is_named_after_its_class() {
        assert!(layout(Language::Java).source.ends_with("Main.java"));
        assert!(layout(Language::Go).source.ends_with("main.go"));
    }

    #[test]
    fn compile_commands_point_only_into_the_job() {
        for lang in [Language::Go, Language::Cpp, Language::Java] {
            let l = layout(lang);
            let argv = lang.compile_argv(&l).expect("compiled language");
            assert!(argv.contains(&s(&l.source)), "{lang:?}: {argv:?}");
            assert!(
                argv.iter()
                    .filter(|a| a.starts_with('/'))
                    .all(|a| a.starts_with("/w/job-1")),
                "{lang:?} references a path outside the job: {argv:?}"
            );
        }
        assert!(Language::Python
            .compile_argv(&layout(Language::Python))
            .is_none());
    }

    #[test]
    fn cpp_is_built_as_asked() {
        let l = layout(Language::Cpp);
        let argv = Language::Cpp.compile_argv(&l).unwrap();
        assert_eq!(&argv[..4], ["g++", "-std=c++17", "-O2", "-pipe"]);
        assert_eq!(
            Language::Cpp.run_argv(&l, 256, Path::new("/t")),
            vec![s(&l.program)]
        );
    }

    #[test]
    fn java_heap_leaves_room_for_the_jvm() {
        let l = layout(Language::Java);
        let argv = Language::Java.run_argv(&l, 256, Path::new("/w/job-1/cases/0"));
        assert_eq!(argv[0], "java");
        assert!(argv.contains(&"-Xmx160m".to_string()), "{argv:?}");
        assert!(argv.contains(&"-Xss64m".to_string()));
        assert!(argv.contains(&"-XX:-UsePerfData".to_string()));
        assert!(argv.contains(&"-Djava.io.tmpdir=/w/job-1/cases/0".to_string()));
        assert_eq!(argv.last().map(String::as_str), Some("Main"));
        assert_eq!(java_heap_mb(32), JVM_MIN_HEAP_MB);
        assert_eq!(java_heap_mb(512), 416);
    }

    #[test]
    fn python_runs_isolated() {
        let l = layout(Language::Python);
        let argv = Language::Python.run_argv(&l, 256, Path::new("/t"));
        assert_eq!(argv[..3], ["python3", "-I", "-B"]);
        assert_eq!(argv[3], s(&l.source));
        assert_eq!(argv.len(), 4);
    }

    #[test]
    fn go_cannot_reach_the_network_or_share_caches() {
        let l = layout(Language::Go);
        let env: std::collections::HashMap<_, _> =
            Language::Go.run_env(&l, 256).into_iter().collect();
        assert_eq!(env["GOTOOLCHAIN"], "local");
        assert_eq!(env["GOPROXY"], "off");
        assert_eq!(env["GO111MODULE"], "off");
        assert_eq!(env["CGO_ENABLED"], "0");
        assert_eq!(env["GOCACHE"], s(&l.go_cache));
        assert_eq!(env["GOPATH"], s(&l.go_path));
        assert_eq!(env["GOMEMLIMIT"], (256 * MIB / 10 * 9).to_string());
        let build: std::collections::HashMap<_, _> =
            Language::Go.compile_env(&l).into_iter().collect();
        assert!(
            !build.contains_key("GOMEMLIMIT"),
            "the compiler must not be squeezed into the program's budget"
        );
    }

    #[test]
    fn only_languages_that_allocate_what_they_use_get_an_address_space_cap() {
        for lang in [Language::Cpp, Language::Python] {
            assert_eq!(lang.run_memory(256).address_space, Some(256 * MIB));
        }
        for lang in [Language::Go, Language::Java] {
            assert_eq!(lang.run_memory(256).address_space, None);
        }
        for lang in Language::ALL {
            assert_eq!(lang.run_memory(256).resident, 256 * MIB);
            assert_eq!(lang.compile_memory(2048).resident, 2048 * MIB);
        }
    }

    #[test]
    fn a_missing_toolchain_probes_as_absent() {
        assert_eq!(
            run_probe(&["definitely-not-a-toolchain-xyz", "--version"]),
            None
        );
        assert_eq!(run_probe(&[]), None);
    }
}
