//! The only module that knows what OS it is on.
//!
//! Everything else in the workspace is plain portable Rust; the differences
//! between Windows, Linux and macOS live here and amount to two things: which
//! binary name to probe for, and suppressing the console window Windows would
//! otherwise flash on every compile.

use crate::RunOutcome;
use dsa_core::problem::{LanguageDef, Toolchain};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The binary to look for on `PATH`, honouring the Windows-specific override
/// (`python3` does not exist there, `python` does).
pub fn probe_name(tc: &Toolchain) -> &str {
    #[cfg(windows)]
    {
        if let Some(p) = &tc.probe_windows {
            return p;
        }
    }
    &tc.probe
}

/// Windows spawns a console window for every child process unless told not to.
/// Compiling three times in a row should not make three black boxes blink.
fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Substitute `{src}`, `{exe}` and `{dir}` in an argv template.
fn expand(args: &[String], src: &Path, exe: &Path, dir: &Path) -> Vec<String> {
    args.iter()
        .map(|a| {
            a.replace("{src}", &src.to_string_lossy())
                .replace("{exe}", &exe.to_string_lossy())
                .replace("{dir}", &dir.to_string_lossy())
        })
        .collect()
}

/// Run a command with stdin, capturing output, killing it after `timeout`.
///
/// `Child::wait_with_output` has no timeout, so the wait happens on a helper
/// thread; if it does not report back in time the child is killed and the
/// thread is left to finish on its own.
fn run_capture(
    argv: &[String],
    cwd: &Path,
    stdin_text: Option<&str>,
    timeout: Duration,
) -> Result<(std::process::Output, bool), String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("empty command".into());
    };

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(if stdin_text.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    quiet(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("could not start {program}: {e}"))?;

    if let Some(text) = stdin_text {
        if let Some(mut sink) = child.stdin.take() {
            // A program that never reads stdin makes this fail with a broken
            // pipe, which is not an error worth reporting.
            let _ = sink.write_all(text.as_bytes());
        }
    }

    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let out = child.wait_with_output();
        let _ = tx.send(out);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            let _ = handle.join();
            Ok((output, false))
        }
        Ok(Err(e)) => Err(format!("{program} failed: {e}")),
        Err(_) => {
            // Timed out. The child is orphaned in the helper thread; kill it
            // by name-independent means: the thread owns it, so ask the OS.
            kill_tree(program);
            Ok((
                std::process::Output {
                    status: Default::default(),
                    stdout: Vec::new(),
                    stderr: b"timed out".to_vec(),
                },
                true,
            ))
        }
    }
}

/// Best-effort kill of a runaway child. The helper thread owns the `Child`
/// handle, so the process is reached through the OS rather than that handle.
fn kill_tree(program: &str) {
    let name = Path::new(program)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| program.to_string());
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("taskkill");
        c.args(["/F", "/T", "/IM", &name]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("pkill");
        c.args(["-f", &name]);
        c
    };
    let _ = quiet(&mut cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Compile (if the language needs it) and run, in a scratch directory that is
/// deleted afterwards.
pub fn run_local(def: &LanguageDef, source: &str, stdin: &str, timeout: Duration) -> RunOutcome {
    let Some(tc) = &def.toolchain else {
        return RunOutcome::failed(format!("{} has no toolchain configured", def.label));
    };

    let dir = match tempfile::Builder::new().prefix("dsa-run-").tempdir() {
        Ok(d) => d,
        Err(e) => return RunOutcome::failed(format!("could not create a scratch directory: {e}")),
    };
    let dir_path = dir.path().to_path_buf();

    // Java insists the file be named after its public class.
    let src_name = def
        .source_name
        .clone()
        .unwrap_or_else(|| format!("main.{}", def.ext));
    let src = dir_path.join(src_name);
    if let Err(e) = std::fs::write(&src, source) {
        return RunOutcome::failed(format!("could not write the source file: {e}"));
    }
    let exe = dir_path.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });

    let mut out = RunOutcome {
        compiled: true,
        ..Default::default()
    };

    if !tc.compile.is_empty() {
        let argv = expand(&tc.compile, &src, &exe, &dir_path);
        match run_capture(&argv, &dir_path, None, timeout) {
            Ok((res, timed_out)) => {
                out.compile_output = format!(
                    "{}{}",
                    String::from_utf8_lossy(&res.stdout),
                    String::from_utf8_lossy(&res.stderr)
                )
                .trim()
                .to_string();
                if timed_out {
                    out.compiled = false;
                    out.timed_out = true;
                    return out;
                }
                if !res.status.success() {
                    out.compiled = false;
                    return out;
                }
            }
            Err(e) => return RunOutcome::failed(e),
        }
    }

    let argv = expand(&tc.run, &src, &exe, &dir_path);
    match run_capture(&argv, &dir_path, Some(stdin), timeout) {
        Ok((res, timed_out)) => {
            out.stdout = String::from_utf8_lossy(&res.stdout).replace("\r\n", "\n");
            out.stderr = String::from_utf8_lossy(&res.stderr).replace("\r\n", "\n");
            out.timed_out = timed_out;
            out.exit_code = if timed_out { None } else { res.status.code() };
        }
        Err(e) => return RunOutcome::failed(e),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::Toolchain;

    fn tc(probe: &str, win: Option<&str>) -> Toolchain {
        Toolchain {
            probe: probe.into(),
            compile: vec![],
            run: vec!["{exe}".into()],
            probe_windows: win.map(|s| s.into()),
        }
    }

    #[test]
    fn windows_probe_override_only_applies_on_windows() {
        let t = tc("python3", Some("python"));
        if cfg!(windows) {
            assert_eq!(probe_name(&t), "python");
        } else {
            assert_eq!(probe_name(&t), "python3");
        }
        assert_eq!(probe_name(&tc("go", None)), "go");
    }

    #[test]
    fn placeholders_expand_and_leave_everything_else_alone() {
        let args: Vec<String> = ["g++", "-O1", "-o", "{exe}", "{src}", "-I{dir}"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = expand(
            &args,
            Path::new("/w/main.cpp"),
            Path::new("/w/prog"),
            Path::new("/w"),
        );
        assert_eq!(out[0], "g++");
        assert_eq!(out[1], "-O1");
        assert_eq!(out[3], "/w/prog");
        assert_eq!(out[4], "/w/main.cpp");
        assert_eq!(out[5], "-I/w");
    }

    #[test]
    fn a_path_with_spaces_stays_one_argument() {
        // No shell is involved, so this must not need quoting.
        let args: Vec<String> = vec!["cc".into(), "{src}".into()];
        let out = expand(
            &args,
            Path::new("C:/Program Files/x/main.c"),
            Path::new(""),
            Path::new(""),
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[1], "C:/Program Files/x/main.c");
    }

    #[test]
    fn a_missing_binary_is_reported_not_panicked() {
        let argv = vec!["definitely-not-a-real-binary-xyz".to_string()];
        let err = run_capture(&argv, Path::new("."), None, Duration::from_secs(1)).unwrap_err();
        assert!(err.contains("could not start"), "{err}");
    }

    #[test]
    fn a_language_without_a_toolchain_fails_cleanly() {
        let def = LanguageDef {
            id: "x".into(),
            label: "X".into(),
            ext: "x".into(),
            syntax: String::new(),
            comment: "//".into(),
            order: 0,
            toolchain: None,
            remote_compiler: None,
            source_name: None,
        };
        let out = run_local(&def, "", "", Duration::from_secs(1));
        assert!(out.error.is_some());
        assert!(!out.ok());
    }
}
