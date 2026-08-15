//! Compiler Explorer fallback.
//!
//! Used only when a language has no local toolchain. It is the one piece of
//! the app that touches the network, and it is entirely optional: without it
//! the practice tab still shows the code and the visualization still runs.

use crate::RunOutcome;
use dsa_core::problem::LanguageDef;
use serde_json::json;
use std::time::Duration;

const API: &str = "https://godbolt.org/api";
const TIMEOUT: Duration = Duration::from_secs(25);

pub fn run_remote(def: &LanguageDef, source: &str, stdin: &str) -> RunOutcome {
    let Some(compiler) = &def.remote_compiler else {
        return RunOutcome::failed(format!("{} has no remote compiler configured", def.label));
    };

    let body = json!({
        "source": source,
        "options": {
            "userArguments": "",
            "executeParameters": { "args": [], "stdin": stdin },
            "compilerOptions": { "executorRequest": true },
            "filters": { "execute": true }
        },
        "lang": def.id,
        "allowStoreCodeDebug": false
    });

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .new_agent();

    let response = agent
        .post(&format!("{API}/compiler/{compiler}/compile"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .send_json(&body);

    let mut res = match response {
        Ok(r) => r,
        Err(e) => {
            return RunOutcome::failed(format!(
            "could not reach Compiler Explorer ({e}). Install a local {} toolchain to run offline.",
            def.label
        ))
        }
    };

    let value: serde_json::Value = match res.body_mut().read_json() {
        Ok(v) => v,
        Err(e) => {
            return RunOutcome::failed(format!("unexpected response from Compiler Explorer: {e}"))
        }
    };
    parse_response(&value)
}

/// Compiler Explorer returns build and execution results in one document;
/// pulling them apart is the only interesting part, and it is pure, so it is
/// tested without touching the network.
pub fn parse_response(v: &serde_json::Value) -> RunOutcome {
    let lines = |key: &str, from: &serde_json::Value| -> String {
        from.get(key)
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|l| l.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    };

    let build = v.get("buildResult").cloned().unwrap_or_else(|| json!({}));
    let build_code = build.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    let compile_output = {
        let a = lines("stdout", &build);
        let b = lines("stderr", &build);
        [a, b]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };

    if build_code != 0 {
        return RunOutcome {
            compiled: false,
            compile_output,
            ..Default::default()
        };
    }

    let code = v.get("code").and_then(|c| c.as_i64()).map(|c| c as i32);
    RunOutcome {
        compiled: true,
        compile_output,
        stdout: lines("stdout", v),
        stderr: lines("stderr", v),
        exit_code: code,
        timed_out: false,
        duration: Duration::ZERO,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_successful_run_is_parsed() {
        let v = json!({
            "code": 0,
            "buildResult": { "code": 0, "stdout": [], "stderr": [] },
            "stdout": [{ "text": "2 4" }],
            "stderr": []
        });
        let out = parse_response(&v);
        assert!(out.compiled);
        assert_eq!(out.output(), "2 4");
        assert_eq!(out.exit_code, Some(0));
        assert!(out.ok());
    }

    #[test]
    fn a_compile_error_keeps_the_diagnostics_and_reports_failure() {
        let v = json!({
            "code": -1,
            "buildResult": {
                "code": 1,
                "stdout": [],
                "stderr": [{ "text": "main.cpp:3:5: error: expected ';'" }]
            }
        });
        let out = parse_response(&v);
        assert!(!out.compiled);
        assert!(out.compile_output.contains("expected ';'"));
        assert!(!out.ok());
    }

    #[test]
    fn a_runtime_failure_is_distinguished_from_a_build_failure() {
        let v = json!({
            "code": 134,
            "buildResult": { "code": 0 },
            "stdout": [],
            "stderr": [{ "text": "panic: index out of range" }]
        });
        let out = parse_response(&v);
        assert!(out.compiled, "it built fine; it died at runtime");
        assert_eq!(out.exit_code, Some(134));
        assert!(out.stderr.contains("panic"));
        assert!(!out.ok());
    }

    #[test]
    fn multi_line_output_is_joined_in_order() {
        let v = json!({
            "code": 0,
            "buildResult": { "code": 0 },
            "stdout": [{ "text": "a" }, { "text": "b" }]
        });
        assert_eq!(parse_response(&v).output(), "a\nb");
    }

    #[test]
    fn a_response_missing_everything_does_not_panic() {
        let out = parse_response(&json!({}));
        assert!(out.compiled, "no buildResult means nothing failed to build");
        assert_eq!(out.output(), "");
    }
}
