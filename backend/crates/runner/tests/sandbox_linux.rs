//! End-to-end sandbox tests: compile and run real programs, benign and
//! hostile, through the full engine.
//!
//! Everything here is Linux-only and skips a language whose toolchain the
//! image does not carry, so the suite is green on a bare Linux checkout and
//! meaningful only where the toolchains (and, for the isolation tests, root)
//! are present — i.e. inside the runner image, which is exactly where it is
//! run in CI (`docker build --target test`).
#![cfg(target_os = "linux")]

use dsa_protocol::{
    CaseInput, CaseStatus, ExecuteRequest, ExecuteResponse, Limits, RunnerErrorCode,
};
use dsa_runner::config::Config;
use dsa_runner::engine::Engine;
use std::sync::OnceLock;
use std::time::Duration;

/// One engine, shared by every test: probing four toolchains per test would
/// dominate the runtime, and the slot pool already isolates concurrent jobs.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        // Non-dumpable + child-subreaper, as `main` does, so the "cannot read
        // the runner's environ" and double-fork-reaping tests exercise the
        // real defences.
        dsa_runner::hardening::harden_process().expect("harden");
        let work = std::env::temp_dir().join(format!("dsa-runner-it-{}", std::process::id()));
        std::fs::create_dir_all(&work).unwrap();
        let vars = [
            ("RUNNER_WORK_DIR", work.to_string_lossy().into_owned()),
            (
                "RUNNER_GO_WARM_CACHE",
                std::env::var("RUNNER_GO_WARM_CACHE")
                    .unwrap_or_else(|_| "/opt/dsa-runner/go-cache".into()),
            ),
        ];
        let config = Config::from_lookup(|k| {
            vars.iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.clone())
        })
        .expect("config");
        Engine::new(config, false).expect("engine")
    })
}

fn have(lang: &str) -> bool {
    engine().health().languages.iter().any(|l| l.id == lang)
}

/// Run a one-case program. Returns `None` (and prints why) when the language
/// is unavailable, so callers can skip cleanly.
fn run(lang: &str, source: &str, stdin: &str, limits: Limits) -> Option<ExecuteResponse> {
    if !have(lang) {
        eprintln!("SKIP: {lang} toolchain not installed");
        return None;
    }
    let req = ExecuteRequest {
        protocol: dsa_protocol::PROTOCOL_VERSION,
        job_id: format!("it-{lang}"),
        language: lang.into(),
        source: source.into(),
        cases: vec![CaseInput {
            id: "c1".into(),
            stdin: stdin.into(),
        }],
        limits,
    };
    let engine = engine();
    let slot = engine.try_acquire().expect("a slot is free");
    Some(futures_block_on(engine.execute(req, &slot)))
}

/// A tiny current-thread block_on, so the tests need no async test harness.
fn futures_block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn fast() -> Limits {
    Limits {
        compile_timeout_ms: 30_000,
        run_timeout_ms: 5_000,
        total_timeout_ms: 40_000,
        memory_mb: 256,
        max_output_bytes: 64 * 1024,
    }
}

/// Assert the program built and its single case got `status`, returning the
/// case so the caller can inspect stdout.
#[track_caller]
fn expect(resp: &ExecuteResponse, status: CaseStatus) -> &dsa_protocol::CaseOutcome {
    assert!(resp.error.is_none(), "runner error: {:?}", resp.error);
    if let Some(c) = &resp.compile {
        assert!(c.ok, "compile failed:\n{}", c.output);
    }
    let case = &resp.cases[0];
    assert_eq!(
        case.status, status,
        "case status\n stdout: {:?}\n stderr: {:?}\n exit: {:?} signal: {:?}",
        case.stdout, case.stderr, case.exit_code, case.signal
    );
    case
}

const LANGS: [&str; 4] = ["go", "cpp", "java", "python"];

fn hello(lang: &str) -> &'static str {
    match lang {
        "go" => "package main\nimport \"fmt\"\nfunc main(){ fmt.Println(\"hello\") }\n",
        "cpp" => "#include <cstdio>\nint main(){ printf(\"hello\\n\"); }\n",
        "java" => "public class Main{ public static void main(String[] a){ System.out.println(\"hello\"); } }\n",
        "python" => "print(\"hello\")\n",
        _ => unreachable!(),
    }
}

/// Two-sum reading `"2 7 11 15\n9\n"` and printing `"0 1"`.
fn two_sum(lang: &str) -> &'static str {
    match lang {
        "go" => {
            r#"package main
import ("bufio";"fmt";"os";"strings";"strconv")
func main(){
    r:=bufio.NewReader(os.Stdin)
    line,_:=r.ReadString('\n')
    var nums []int
    for _,f:=range strings.Fields(line){ n,_:=strconv.Atoi(f); nums=append(nums,n) }
    var t int; fmt.Fscan(r,&t)
    seen:=map[int]int{}
    for i,x:=range nums{ if j,ok:=seen[t-x];ok{ fmt.Printf("%d %d\n",j,i); return }; seen[x]=i }
}
"#
        }
        "cpp" => {
            r#"#include <bits/stdc++.h>
using namespace std;
int main(){
    string line; getline(cin,line); istringstream ss(line);
    vector<int> nums; int v; while(ss>>v) nums.push_back(v);
    int t; cin>>t; unordered_map<int,int> seen;
    for(int i=0;i<(int)nums.size();++i){ auto it=seen.find(t-nums[i]); if(it!=seen.end()){ cout<<it->second<<" "<<i<<"\n"; return 0; } seen[nums[i]]=i; }
}
"#
        }
        "java" => {
            r#"import java.util.*;import java.io.*;
public class Main{ public static void main(String[] a) throws IOException{
    BufferedReader br=new BufferedReader(new InputStreamReader(System.in));
    int[] nums=Arrays.stream(br.readLine().trim().split("\\s+")).mapToInt(Integer::parseInt).toArray();
    int t=Integer.parseInt(br.readLine().trim());
    Map<Integer,Integer> seen=new HashMap<>();
    for(int i=0;i<nums.length;i++){ Integer j=seen.get(t-nums[i]); if(j!=null){ System.out.println(j+" "+i); return; } seen.put(nums[i],i); }
} }
"#
        }
        "python" => {
            r#"nums=list(map(int,input().split()))
t=int(input())
seen={}
for i,x in enumerate(nums):
    if t-x in seen:
        print(seen[t-x],i); break
    seen[x]=i
"#
        }
        _ => unreachable!(),
    }
}

#[test]
fn hello_world_compiles_and_runs_in_every_language() {
    for lang in LANGS {
        let Some(resp) = run(lang, hello(lang), "", fast()) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::Ok);
        assert_eq!(case.stdout.trim(), "hello", "{lang}");
        assert_eq!(case.exit_code, Some(0), "{lang}");
        assert_eq!(
            resp.compile.is_none(),
            lang == "python",
            "{lang} compile phase"
        );
    }
}

#[test]
fn two_sum_reads_the_harness_stdin_format() {
    for lang in LANGS {
        let Some(resp) = run(lang, two_sum(lang), "2 7 11 15\n9\n", fast()) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::Ok);
        assert_eq!(case.stdout.trim(), "0 1", "{lang}");
    }
}

#[test]
fn a_compile_error_is_reported_not_crashed() {
    // Missing semicolon / brace per language; Python has no compile step, so
    // its syntax error is a per-case runtime error instead.
    let broken = [
        ("go", "package main\nfunc main(){ this is not go }\n"),
        ("cpp", "int main(){ return }\n"),
        ("java", "public class Main{ void oops( }\n"),
    ];
    for (lang, src) in broken {
        let Some(resp) = run(lang, src, "", fast()) else {
            continue;
        };
        assert!(resp.error.is_none(), "{lang}: {:?}", resp.error);
        let compile = resp.compile.expect("compiled language");
        assert!(!compile.ok, "{lang} should not compile");
        assert!(!compile.output.is_empty(), "{lang} compiler said nothing");
        assert!(
            resp.cases.is_empty(),
            "{lang} ran cases despite a build error"
        );
    }

    if let Some(resp) = run("python", "def f(:\n", "", fast()) {
        assert!(resp.compile.is_none());
        assert_eq!(resp.cases[0].status, CaseStatus::RuntimeError);
    }
}

#[test]
fn an_infinite_loop_times_out() {
    let progs = [
        ("go", "package main\nfunc main(){ for{} }\n"),
        ("cpp", "int main(){ for(;;){} }\n"),
        ("python", "while True: pass\n"),
    ];
    let limits = Limits {
        run_timeout_ms: 1000,
        ..fast()
    };
    for (lang, src) in progs {
        let Some(resp) = run(lang, src, "", limits) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::Timeout);
        assert!(
            case.duration_ms >= 900 && case.duration_ms < 4000,
            "{lang}: {}ms",
            case.duration_ms
        );
    }
}

#[test]
fn a_memory_hog_is_stopped_and_the_runner_survives() {
    let progs = [
        ("python", "a=[]\nwhile True:\n    a.append(bytearray(16*1024*1024))\n"),
        ("cpp", "#include <vector>\nint main(){ std::vector<char*> v; while(true){ auto p=new char[16*1024*1024]; for(int i=0;i<16*1024*1024;i+=4096)p[i]=1; v.push_back(p); } }\n"),
        ("go", "package main\nfunc main(){ var a [][]byte; for{ b:=make([]byte,16*1024*1024); for i:=range b{ b[i]=1 }; a=append(a,b) } }\n"),
        ("java", "public class Main{ public static void main(String[] x){ java.util.List<byte[]> a=new java.util.ArrayList<>(); while(true){ a.add(new byte[16*1024*1024]); } } }\n"),
    ];
    for (lang, src) in progs {
        let Some(resp) = run(
            lang,
            src,
            "",
            Limits {
                memory_mb: 128,
                run_timeout_ms: 5000,
                ..fast()
            },
        ) else {
            continue;
        };
        let case = expect_no_runner_error(&resp);
        assert!(
            matches!(
                case.status,
                CaseStatus::MemoryLimit | CaseStatus::RuntimeError
            ),
            "{lang}: expected memory limit, got {:?} (stderr {:?})",
            case.status,
            case.stderr
        );
    }
    // The runner is still healthy afterwards.
    let ok = run("python", "print(1+1)", "", fast()).unwrap();
    assert_eq!(expect(&ok, CaseStatus::Ok).stdout.trim(), "2");
}

#[test]
fn an_output_flood_hits_the_output_limit() {
    let progs = [
        ("python", "import sys\nwhile True: sys.stdout.write('x'*4096)\n"),
        ("cpp", "#include <cstdio>\nint main(){ char b[4096]; for(int i=0;i<4096;i++)b[i]='x'; for(;;) fwrite(b,1,4096,stdout); }\n"),
    ];
    for (lang, src) in progs {
        let Some(resp) = run(
            lang,
            src,
            "",
            Limits {
                max_output_bytes: 32 * 1024,
                ..fast()
            },
        ) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::OutputLimit);
        assert!(case.stdout_truncated, "{lang}");
        assert_eq!(case.stdout.len(), 32 * 1024, "{lang}: kept exactly the cap");
    }
}

#[test]
fn a_fork_bomb_is_contained_and_the_runner_survives() {
    // Only meaningful with per-uid RLIMIT_NPROC, i.e. as root.
    if !is_root() {
        eprintln!("SKIP: fork-bomb containment needs root (per-uid process limit)");
        return;
    }
    let progs = [
        (
            "python",
            "import os\nwhile True:\n    try: os.fork()\n    except Exception: pass\n",
        ),
        (
            "cpp",
            "#include <unistd.h>\nint main(){ while(true) fork(); }\n",
        ),
    ];
    for (lang, src) in progs {
        let Some(resp) = run(
            lang,
            src,
            "",
            Limits {
                run_timeout_ms: 1500,
                ..fast()
            },
        ) else {
            continue;
        };
        let case = expect_no_runner_error(&resp);
        // However it is stopped — the process limit exhausting fork attempts
        // (timeout), a child dying (runtime error), or the forked interpreters'
        // combined RSS tripping the memory watchdog — the point is that it is
        // stopped and bounded.
        assert!(
            matches!(
                case.status,
                CaseStatus::Timeout | CaseStatus::RuntimeError | CaseStatus::MemoryLimit
            ),
            "{lang}: {:?}",
            case.status
        );
    }
    // The runner is unharmed: a normal job still works.
    let ok = run("python", "print('alive')", "", fast()).unwrap();
    assert_eq!(expect(&ok, CaseStatus::Ok).stdout.trim(), "alive");
}

#[test]
fn opening_a_network_socket_is_refused() {
    let progs = [
        (
            "python",
            r#"import socket
try:
    s=socket.socket(socket.AF_INET, socket.SOCK_STREAM); s.settimeout(2)
    s.connect(("1.1.1.1",80)); print("OPEN")
except OSError as e:
    print("blocked", e.errno)
"#,
        ),
        (
            "go",
            r#"package main
import ("fmt";"net";"time")
func main(){
    _,err:=net.DialTimeout("tcp","1.1.1.1:80",2*time.Second)
    if err!=nil { fmt.Println("blocked") } else { fmt.Println("OPEN") }
}
"#,
        ),
        (
            "cpp",
            r#"#include <sys/socket.h>
#include <cstdio>
int main(){ int fd=socket(AF_INET,SOCK_STREAM,0); printf("%s\n", fd<0?"blocked":"OPEN"); }
"#,
        ),
    ];
    for (lang, src) in progs {
        let Some(resp) = run(lang, src, "", fast()) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::Ok);
        assert!(case.stdout.contains("blocked"), "{lang}: {:?}", case.stdout);
        assert!(!case.stdout.contains("OPEN"), "{lang} reached the network");
    }
}

#[test]
fn local_unix_sockets_still_work() {
    // AF_UNIX is allowed (runtimes use it locally); only the reach outside the
    // machine is cut. This guards against over-blocking `socket`.
    let src = r#"import socket
a,b=socket.socketpair()
a.send(b"ping"); print(b.recv(4).decode())
s=socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); print("unix-ok")
"#;
    if let Some(resp) = run("python", src, "", fast()) {
        let case = expect(&resp, CaseStatus::Ok);
        assert!(
            case.stdout.contains("ping"),
            "socketpair: {:?}",
            case.stdout
        );
        assert!(
            case.stdout.contains("unix-ok"),
            "AF_UNIX socket refused: {:?}",
            case.stdout
        );
    }
}

#[test]
fn reading_other_processes_secrets_is_refused() {
    // pid 1's environment, and the runner's own (the child's parent).
    let src = r#"import os
def blocked(path):
    try:
        with open(path,"rb") as f: f.read(1); return False
    except OSError: return True
ppid=os.getppid()
print("init", blocked("/proc/1/environ"))
print("runner", blocked(f"/proc/{ppid}/environ"))
"#;
    if let Some(resp) = run("python", src, "", fast()) {
        let case = expect(&resp, CaseStatus::Ok);
        assert!(
            case.stdout.contains("init True"),
            "read /proc/1/environ: {:?}",
            case.stdout
        );
        assert!(
            case.stdout.contains("runner True"),
            "read the runner's environ: {:?}",
            case.stdout
        );
    }
}

#[test]
fn writing_outside_the_job_directory_is_refused() {
    // Its own case directory is writable; the job tree above it and system
    // directories are not. (World-writable /tmp is out of scope for a
    // namespace-less sandbox; see the README.)
    let src = r#"import os
def blocked(path):
    try:
        open(path,"w").close(); os.remove(path); return False
    except OSError: return True
open("mine","w").write("ok")           # cwd is this case's dir
print("cwd", os.path.exists("mine"))
print("etc", blocked("/etc/dsa-evil"))
print("parent", blocked("../escape"))
print("root", blocked("/dsa-evil"))
"#;
    if !is_root() {
        eprintln!("SKIP: filesystem ownership isolation needs root");
        return;
    }
    if let Some(resp) = run("python", src, "", fast()) {
        let case = expect(&resp, CaseStatus::Ok);
        assert!(
            case.stdout.contains("cwd True"),
            "own dir not writable: {:?}",
            case.stdout
        );
        for probe in ["etc True", "parent True", "root True"] {
            assert!(case.stdout.contains(probe), "{probe}: {:?}", case.stdout);
        }
    }
}

#[test]
fn a_backgrounded_daemon_is_killed_before_it_can_act() {
    if !is_root() {
        eprintln!("SKIP: daemon reaping is asserted against a per-uid scan (root)");
        return;
    }
    let marker = format!("/tmp/dsa-daemon-{}", uuid_like());
    // Double-fork + setsid: the daemon leaves the process group, so only the
    // reaper (per-uid scan) can catch it. It would create `marker` after 3s.
    let src = format!(
        r#"import os,sys,time
pid=os.fork()
if pid==0:
    os.setsid()
    if os.fork()==0:
        time.sleep(3)
        open({marker:?},"w").write("escaped")
        os._exit(0)
    os._exit(0)
os.waitpid(pid,0)
print("spawned")
"#
    );
    if let Some(resp) = run("python", &src, "", fast()) {
        assert_eq!(expect(&resp, CaseStatus::Ok).stdout.trim(), "spawned");
    } else {
        return;
    }
    // Well past the daemon's sleep: the marker must never appear.
    std::thread::sleep(Duration::from_secs(5));
    assert!(
        !std::path::Path::new(&marker).exists(),
        "a double-forked daemon survived teardown and wrote {marker}"
    );
}

#[test]
fn deep_recursion_within_the_stack_limit_succeeds() {
    // ~15 MiB of frames: over the default 8 MiB stack (would segfault), under
    // the sandbox's 64 MiB. Proves RLIMIT_STACK is raised, not just inherited.
    let n = 60_000i64;
    let expected = n * (n + 1) / 2;
    // The 256-byte volatile frame (touched before and after the recursive call)
    // defeats tail-call optimisation, so the recursion really consumes stack;
    // `buf[0]` is always 1, so the arithmetic stays correct.
    let src = format!(
        r#"#include <cstdio>
long long f(int n){{
    volatile char buf[256];
    buf[0] = 1;
    if (n == 0) return 0;
    long long r = (long long)n + f(n - 1);
    return r + (long long)(buf[0] - 1);
}}
int main(){{ printf("%lld\n", f({n})); }}
"#
    );
    if let Some(resp) = run("cpp", &src, "", fast()) {
        let case = expect(&resp, CaseStatus::Ok);
        assert_eq!(case.stdout.trim(), expected.to_string());
    }
}

#[test]
fn a_runtime_crash_is_a_runtime_error_not_a_runner_error() {
    let progs = [
        ("python", "raise SystemExit(3)\n", Some(3)),
        ("cpp", "int main(){ int*p=nullptr; return *p; }\n", None),
        (
            "go",
            "package main\nfunc main(){ var s []int; _ = s[5] }\n",
            Some(2),
        ),
    ];
    for (lang, src, code) in progs {
        let Some(resp) = run(lang, src, "", fast()) else {
            continue;
        };
        let case = expect(&resp, CaseStatus::RuntimeError);
        if let Some(expected) = code {
            assert_eq!(case.exit_code, Some(expected), "{lang}");
        }
    }
}

#[test]
fn an_unknown_language_is_refused_cleanly() {
    let engine = engine();
    let req = ExecuteRequest {
        protocol: dsa_protocol::PROTOCOL_VERSION,
        job_id: "it-unknown".into(),
        language: "rust".into(),
        source: "fn main(){}".into(),
        cases: vec![CaseInput {
            id: "c".into(),
            stdin: String::new(),
        }],
        limits: fast(),
    };
    let slot = engine.try_acquire().unwrap();
    let resp = futures_block_on(engine.execute(req, &slot));
    assert_eq!(
        resp.error.map(|e| e.code),
        Some(RunnerErrorCode::UnsupportedLanguage)
    );
}

#[track_caller]
fn expect_no_runner_error(resp: &ExecuteResponse) -> &dsa_protocol::CaseOutcome {
    assert!(resp.error.is_none(), "runner error: {:?}", resp.error);
    if let Some(c) = &resp.compile {
        assert!(c.ok, "compile failed:\n{}", c.output);
    }
    &resp.cases[0]
}

fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() == 0 }
}

fn uuid_like() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
