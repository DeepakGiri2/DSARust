//! Compiling and running practice submissions.
//!
//! Two backends, chosen per language at startup:
//!
//! * **Local** — whatever toolchain is on `PATH`, driven through the argv
//!   templates in `content/languages.toml`. No shell is involved anywhere, so
//!   paths with spaces (the default on Windows) behave the same on every OS.
//! * **Remote** — the Compiler Explorer API, used when the toolchain is not
//!   installed. Needs the network; nothing else in the app does.
//!
//! Everything platform-specific is confined to [`platform`], which is the only
//! module with a `#[cfg]` on it.

pub mod platform;
pub mod remote;
pub mod tests;

use dsa_core::problem::{InputField, InputMap, LangId, LanguageDef};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use tests::{outputs_match, run_tests, summary, TestOutcome, TestStatus};

/// How long a single run may take before it is killed.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Local,
    Remote,
    /// Neither a toolchain nor a remote compiler id — viewer only.
    Unavailable,
}

impl Backend {
    pub fn label(&self) -> &'static str {
        match self {
            Backend::Local => "local toolchain",
            Backend::Remote => "Compiler Explorer",
            Backend::Unavailable => "not runnable",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunOutcome {
    pub compiled: bool,
    pub compile_output: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration: Duration,
    /// Set when the harness itself failed (no toolchain, network down, ...).
    pub error: Option<String>,
}

impl RunOutcome {
    pub fn failed(msg: impl Into<String>) -> Self {
        Self {
            error: Some(msg.into()),
            ..Default::default()
        }
    }
    pub fn ok(&self) -> bool {
        self.error.is_none() && self.compiled && !self.timed_out && self.exit_code == Some(0)
    }
    /// What the tests compare against.
    pub fn output(&self) -> &str {
        self.stdout.trim_end()
    }
}

/// Resolved execution capability for every declared language.
pub struct Harness {
    languages: Vec<LanguageDef>,
    /// Absolute path of each language's probe binary, when it was found.
    local: BTreeMap<LangId, PathBuf>,
    pub allow_remote: bool,
}

impl Harness {
    /// Probes `PATH` once. Cheap, but not free, so the app keeps one instance.
    pub fn detect(languages: &[LanguageDef]) -> Self {
        let mut local = BTreeMap::new();
        for lang in languages {
            if let Some(tc) = &lang.toolchain {
                let probe = platform::probe_name(tc);
                if let Ok(path) = which::which(probe) {
                    log::info!("{}: found {} at {}", lang.id, probe, path.display());
                    local.insert(lang.id.clone(), path);
                }
            }
        }
        Self {
            languages: languages.to_vec(),
            local,
            allow_remote: true,
        }
    }

    pub fn language(&self, id: &str) -> Option<&LanguageDef> {
        self.languages.iter().find(|l| l.id == id)
    }

    pub fn backend(&self, lang: &str) -> Backend {
        if self.local.contains_key(lang) {
            Backend::Local
        } else if self.allow_remote
            && self
                .language(lang)
                .is_some_and(|l| l.remote_compiler.is_some())
        {
            Backend::Remote
        } else {
            Backend::Unavailable
        }
    }

    /// Human-readable summary for the practice tab's status line.
    pub fn describe(&self, lang: &str) -> String {
        match self.backend(lang) {
            Backend::Local => {
                let p = self
                    .local
                    .get(lang)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                format!("local toolchain — {p}")
            }
            Backend::Remote => {
                "no local toolchain — will compile on Compiler Explorer (needs internet)".into()
            }
            Backend::Unavailable => "no toolchain and no remote compiler configured".into(),
        }
    }

    pub fn run(&self, lang: &str, source: &str, stdin: &str) -> RunOutcome {
        self.run_with_timeout(lang, source, stdin, DEFAULT_TIMEOUT)
    }

    pub fn run_with_timeout(
        &self,
        lang: &str,
        source: &str,
        stdin: &str,
        timeout: Duration,
    ) -> RunOutcome {
        let started = Instant::now();
        let Some(def) = self.language(lang) else {
            return RunOutcome::failed(format!("unknown language \"{lang}\""));
        };

        let mut out = match self.backend(lang) {
            Backend::Local => platform::run_local(def, source, stdin, timeout),
            Backend::Remote => remote::run_remote(def, source, stdin),
            Backend::Unavailable => RunOutcome::failed(format!(
                "{} cannot be run here: install {} or configure a remote compiler",
                def.label,
                def.toolchain
                    .as_ref()
                    .map(|t| t.probe.as_str())
                    .unwrap_or("a toolchain")
            )),
        };
        out.duration = started.elapsed();
        out
    }
}

/// Serialize an input map to the stdin the harness programs expect: one field
/// per line, in the order the problem declares them.
///
/// This is the contract between a problem's `main()` and its test cases, so it
/// deliberately matches the original TypeScript app byte for byte.
pub fn serialize_input(fields: &[InputField], input: &InputMap) -> String {
    let mut out = String::new();
    for f in fields {
        if let Some(v) = input.get(&f.name) {
            out.push_str(&v.to_editable());
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod lib_tests {
    use super::*;
    use dsa_core::problem::{InputType, InputValue};

    fn field(name: &str, ty: InputType) -> InputField {
        InputField {
            name: name.into(),
            label: String::new(),
            ty,
            min: None,
            max: None,
            min_len: None,
            max_len: None,
            charset: None,
            sorted: false,
            unique: false,
            help: None,
        }
    }

    #[test]
    fn stdin_is_one_field_per_line_in_declaration_order() {
        let fields = vec![
            field("nums", InputType::IntArray),
            field("target", InputType::Int),
        ];
        let mut input = InputMap::new();
        // Inserted in the opposite order on purpose: the schema decides.
        input.insert("target".into(), InputValue::Int(9));
        input.insert(
            "nums".into(),
            InputValue::List(vec![InputValue::Int(2), InputValue::Int(7)]),
        );
        assert_eq!(serialize_input(&fields, &input), "2 7\n9\n");
    }

    #[test]
    fn a_missing_field_still_emits_its_line() {
        // Otherwise every later field would shift up a line in the program's
        // reader and the failure would look like a wrong answer.
        let fields = vec![field("a", InputType::Int), field("b", InputType::Int)];
        let mut input = InputMap::new();
        input.insert("b".into(), InputValue::Int(3));
        assert_eq!(serialize_input(&fields, &input), "\n3\n");
    }

    #[test]
    fn outcome_ok_requires_everything_to_have_gone_right() {
        let good = RunOutcome {
            compiled: true,
            exit_code: Some(0),
            ..Default::default()
        };
        assert!(good.ok());
        assert!(!RunOutcome {
            compiled: false,
            ..good.clone()
        }
        .ok());
        assert!(!RunOutcome {
            exit_code: Some(1),
            ..good.clone()
        }
        .ok());
        assert!(!RunOutcome {
            timed_out: true,
            ..good.clone()
        }
        .ok());
        assert!(!RunOutcome::failed("boom").ok());
    }

    #[test]
    fn backend_falls_back_from_local_to_remote_to_nothing() {
        let langs = vec![
            LanguageDef {
                id: "nope".into(),
                label: "Nope".into(),
                ext: "np".into(),
                syntax: String::new(),
                comment: "//".into(),
                order: 0,
                toolchain: None,
                remote_compiler: Some("x".into()),
                source_name: None,
            },
            LanguageDef {
                id: "none".into(),
                label: "None".into(),
                ext: "nn".into(),
                syntax: String::new(),
                comment: "//".into(),
                order: 1,
                toolchain: None,
                remote_compiler: None,
                source_name: None,
            },
        ];
        let h = Harness::detect(&langs);
        assert_eq!(h.backend("nope"), Backend::Remote);
        assert_eq!(h.backend("none"), Backend::Unavailable);
        assert_eq!(h.backend("missing-language"), Backend::Unavailable);
    }

    #[test]
    fn disabling_remote_makes_a_toolchainless_language_unavailable() {
        let langs = vec![LanguageDef {
            id: "r".into(),
            label: "R".into(),
            ext: "r".into(),
            syntax: String::new(),
            comment: "#".into(),
            order: 0,
            toolchain: None,
            remote_compiler: Some("x".into()),
            source_name: None,
        }];
        let mut h = Harness::detect(&langs);
        h.allow_remote = false;
        assert_eq!(h.backend("r"), Backend::Unavailable);
    }
}
