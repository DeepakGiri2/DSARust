//! The ✏ Practice tab: write the solution yourself, run it, test it, and ask
//! the local AI for help.
//!
//! Four columns, as in the web version: the problem statement, the editor,
//! stdin/output/tests, and the AI panel — the outer two collapsible from the
//! toolbar.

use crate::assistant::{AiAction, Assistant, Brief};
use crate::progress::Progress;
use crate::settings::Settings;
use crate::style::*;
use dsa_content::Library;
use dsa_core::diff::{Change, Diff, Row};
use dsa_harness::{
    run_tests, serialize_input, Backend, Harness, RunOutcome, TestOutcome, TestStatus,
};
use egui::{Align, Layout, RichText, ScrollArea, Ui};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{channel, Receiver};

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Solution,
    Full,
}

enum Job {
    Run(RunOutcome),
    Tests(Vec<TestOutcome>),
}

/// An AI fix being reviewed.
///
/// The editor is *not* overwritten while this is alive. The user's code stays
/// exactly as they left it and the centre column shows the proposal diffed
/// against it — which is the whole point, since a silently replaced buffer
/// hands you working code and hides the mistake you made. Discarding is
/// therefore free: there is nothing to put back.
struct Review {
    /// The (problem, language) the proposal was written for. Switching language
    /// with a review open would otherwise splice a Go fix into the C++ buffer:
    /// the banner applies to whatever `key` the frame is holding.
    key: (String, String),
    diff: Diff,
    /// One flag per change group; all on when the fix arrives.
    accepted: Vec<bool>,
}

impl Review {
    fn accepted_count(&self) -> usize {
        self.accepted.iter().filter(|a| **a).count()
    }
}

pub struct Practice {
    /// Editor buffers per (slug, language) so switching tabs never loses work.
    solutions: BTreeMap<(String, String), String>,
    full: BTreeMap<(String, String), String>,
    /// Which full-program buffers the user has edited. An untouched one is
    /// regenerated from the solution, so it never goes stale; an edited one is
    /// theirs and is what runs.
    full_edited: BTreeSet<(String, String)>,
    view: View,
    stdin: String,
    stdin_for: Option<(String, String)>,
    result: Option<RunOutcome>,
    tests: Vec<TestOutcome>,
    running: bool,
    pending: Option<Receiver<Job>>,
    review: Option<Review>,
    pub assistant: Assistant,
}

impl Default for Practice {
    fn default() -> Self {
        Self {
            solutions: BTreeMap::new(),
            full: BTreeMap::new(),
            full_edited: BTreeSet::new(),
            view: View::Solution,
            stdin: String::new(),
            stdin_for: None,
            result: None,
            tests: Vec::new(),
            running: false,
            pending: None,
            review: None,
            assistant: Assistant::default(),
        }
    }
}

/// Whether a finished job earned the problem a tick.
///
/// Green only for a full sweep: every case run, every case passed. One passing
/// case out of three is not a solution, and an empty test list is not a pass.
pub fn all_tests_passed(outcomes: &[TestOutcome], expected: usize) -> bool {
    expected > 0 && outcomes.len() == expected && outcomes.iter().all(|o| o.status.passed())
}

impl Practice {
    /// Poll the background compiler. Returns whether the frame is dirty, and
    /// records progress for `slug` when a job comes back.
    pub fn poll(
        &mut self,
        settings: &mut Settings,
        progress: &mut Progress,
        slug: &str,
        test_count: usize,
    ) -> bool {
        let mut dirty = self.assistant.poll(settings);
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(Job::Run(out)) => {
                    self.result = Some(out);
                    self.running = false;
                    self.pending = None;
                    dirty = true;
                }
                Ok(Job::Tests(t)) => {
                    // Passing every case is the one signal worth trusting, so
                    // it is the one that ticks the box without being asked.
                    if all_tests_passed(&t, test_count) {
                        progress.mark_solved(slug);
                    }
                    self.tests = t;
                    self.running = false;
                    self.pending = None;
                    dirty = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.running = false;
                    self.pending = None;
                    dirty = true;
                }
            }
        }
        dirty || self.running
    }

    /// Everything the AI's Fix mode should know about the last run.
    fn run_context(&self, fields: &[dsa_core::problem::InputField]) -> String {
        let mut parts = Vec::new();
        if let Some(r) = &self.result {
            if let Some(e) = &r.error {
                parts.push(format!("run error: {e}"));
            }
            if !r.compiled {
                parts.push(format!(
                    "compile error:\n{}",
                    truncate(&r.compile_output, 1200)
                ));
            } else {
                if !r.stderr.trim().is_empty() {
                    parts.push(format!("stderr:\n{}", truncate(&r.stderr, 800)));
                }
                if !r.stdout.trim().is_empty() {
                    parts.push(format!("stdout:\n{}", truncate(&r.stdout, 800)));
                }
                if r.exit_code.is_some_and(|c| c != 0) {
                    parts.push(format!("exit code: {}", r.exit_code.unwrap()));
                }
            }
        }
        for (i, t) in self.tests.iter().enumerate() {
            match t.status {
                TestStatus::Failed => parts.push(format!(
                    "test {} FAILED — input: {} · expected \"{}\" · got \"{}\"",
                    i + 1,
                    t.stdin.trim().replace('\n', " | "),
                    t.expected,
                    if t.actual.is_empty() {
                        "(nothing)"
                    } else {
                        &t.actual
                    }
                )),
                TestStatus::Passed => {}
                _ => parts.push(format!(
                    "test {} {} — {}",
                    i + 1,
                    t.status.label(),
                    truncate(&t.detail, 400)
                )),
            }
        }
        let _ = fields;
        parts.join("\n")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        lib: &Library,
        harness: &Harness,
        settings: &mut Settings,
        progress: &mut Progress,
        slug: &str,
        lang: &str,
    ) {
        let Some(pack) = lib.pack(slug) else {
            side_empty(ui, "No content pack for this problem.");
            return;
        };
        let key = (slug.to_string(), lang.to_string());

        // Seed the editor from the pack's starter the first time it is opened.
        if !self.solutions.contains_key(&key) {
            self.solutions.insert(key.clone(), starter_for(pack, lang));
        }
        // And seed the full-program buffer for *this* pair. Doing it only on a
        // view switch left the editor blank whenever the language changed while
        // Full was showing, because the new pair had no buffer to show.
        if self.view == View::Full && !self.full.contains_key(&key) {
            let assembled = self.assembled(&key, pack, lang);
            self.full.insert(key.clone(), assembled);
        }
        // A language switch is a different program with different output, so
        // the previous run's result and test results no longer describe what is
        // on screen.
        if self.stdin_for.as_ref() != Some(&key) {
            let same_problem = self.stdin_for.as_ref().is_some_and(|(s, _)| s == slug);
            if !same_problem {
                self.stdin = serialize_input(&pack.meta.inputs, &pack.meta.default_input);
            }
            self.stdin_for = Some(key.clone());
            self.result = None;
            self.tests.clear();
        }

        let lang_label = lib
            .language(lang)
            .map(|l| l.label.clone())
            .unwrap_or_else(|| lang.into());
        let syntax = lib
            .language(lang)
            .map(|l| l.syntax.clone())
            .unwrap_or_else(|| lang.into());

        // A proposal is written against one language's solution. Changing
        // language while it is open leaves it describing a file that is no
        // longer on screen, so it goes.
        if self.review.as_ref().is_some_and(|r| r.key != key) {
            self.review = None;
        }

        // Pressing Run or the test button is what "attempted" means; the
        // outcome, which decides "solved", lands later in `poll`.
        if self.toolbar(ui, settings, harness, lang, &lang_label, &key, pack) {
            progress.record_attempt(slug);
        }
        if self.review.is_some() {
            self.fix_banner(ui, &key);
        }
        ui.add_space(4.0);

        // ── columns ─────────────────────────────────────────────────────────
        if settings.show_question {
            egui::SidePanel::left("q-panel")
                .resizable(true)
                .default_width(340.0)
                .width_range(240.0..=520.0)
                .show_inside(ui, |ui| question_panel(ui, pack));
        }
        if settings.ai_open {
            let brief = brief_for(lib, pack);
            let code = self.solutions.get(&key).cloned().unwrap_or_default();
            let context = self.run_context(&pack.meta.inputs);
            let mut proposed: Option<String> = None;
            egui::SidePanel::right("ai-panel")
                .resizable(true)
                .default_width(380.0)
                .width_range(300.0..=560.0)
                .show_inside(ui, |ui| {
                    if let AiAction::ProposeFix(fixed) =
                        self.assistant
                            .ui(ui, settings, &brief, lang, &lang_label, &code, &context)
                    {
                        proposed = Some(fixed);
                    }
                });
            if let Some(fixed) = proposed {
                self.start_fix_review(&key, fixed);
            }
        }
        egui::SidePanel::right("io-panel")
            .resizable(true)
            .default_width(320.0)
            .width_range(240.0..=520.0)
            .show_inside(ui, |ui| self.io_panel(ui, pack, &lang_label));

        let mut edited = false;
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // A review owns the centre column: while one is open the editor is
            // replaced by the change itself, in red and green.
            if let Some(review) = &mut self.review {
                diff_view(ui, review);
                return;
            }
            let buffer = match self.view {
                View::Solution => self.solutions.entry(key.clone()).or_default(),
                View::Full => self.full.entry(key.clone()).or_default(),
            };
            edited = editor(ui, buffer, &syntax).changed();
        });
        if edited && self.view == View::Full {
            self.full_edited.insert(key.clone());
        }
    }

    /// Returns whether this frame started a run — one attempt, either button.
    #[allow(clippy::too_many_arguments)]
    fn toolbar(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        harness: &Harness,
        lang: &str,
        lang_label: &str,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
    ) -> bool {
        let backend = harness.backend(lang);
        // In the solution view there has to be something to wrap the solution
        // in. Without it Run would hand the compiler a bare function and get
        // back a complaint about `package` — a message about the harness, shown
        // to someone looking at their own code.
        let wrapped = self.view == View::Full || harness_for(pack, lang).is_some();
        let runnable =
            backend != Backend::Unavailable && wrapped && !self.running && self.review.is_none();
        let mut attempted = false;

        toolbar(ui, |ui| {
            let label = if self.running {
                "⏳ running…"
            } else {
                "▶ Run"
            };
            if apply_btn(ui, label, runnable).clicked() {
                self.spawn_run(harness, lang, key, pack);
                attempted = true;
            }
            if ui
                .add_enabled(
                    runnable && !pack.meta.tests.is_empty(),
                    egui::Button::new(RichText::new("✔ run tests").size(12.0).color(TEXT_DIM))
                        .fill(PANEL2)
                        .stroke(egui::Stroke::new(1.0, BORDER)),
                )
                .clicked()
            {
                self.spawn_tests(harness, lang, key, pack);
                attempted = true;
            }

            let mut view = self.view;
            if seg(
                ui,
                &mut view,
                &[(View::Solution, "solution"), (View::Full, "full program")],
            ) {
                self.switch_view(view, key, pack, lang);
            }

            if mini_btn(ui, "reset code")
                .on_hover_text(match self.view {
                    View::Full => "Regenerate the full program from your solution",
                    View::Solution => "Reset to the empty starter skeleton",
                })
                .clicked()
            {
                self.reset_code(key, pack, lang);
            }

            if wrapped {
                ui.label(
                    RichText::new(log_hint(lang))
                        .monospace()
                        .size(11.0)
                        .color(TEXT_DIM),
                );
            } else {
                // Say what is missing and where to go, rather than letting Run
                // fail with a compiler message about someone else's code.
                ui.label(
                    RichText::new("⚠ no runnable harness for this problem")
                        .size(11.5)
                        .color(AMBER),
                )
                .on_hover_text(
                    "This pack ships no main() for this language, and its inputs do not line up \
                     with the solution's parameters closely enough to derive one. Switch to \
                     “full program” to write the whole thing, including main().",
                );
                if mini_btn(ui, "→ full program").clicked() {
                    self.switch_view(View::Full, key, pack, lang);
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle_btn(ui, "💬 AI assist", settings.ai_open)
                    .on_hover_text("Local AI assistant (Ollama) — interview, guide or fix mode")
                    .clicked()
                {
                    settings.ai_open = !settings.ai_open;
                    if settings.ai_open {
                        let url = settings.ollama_url.clone();
                        self.assistant.connect(&url);
                    }
                }
                if toggle_btn(ui, "📄 question", settings.show_question)
                    .on_hover_text("Show / hide the problem statement")
                    .clicked()
                {
                    settings.show_question = !settings.show_question;
                }
                let (text, color) = match backend {
                    Backend::Local => (format!("{lang_label} · local toolchain"), GREEN),
                    Backend::Remote => (format!("{lang_label} · Compiler Explorer"), AMBER),
                    Backend::Unavailable => (format!("{lang_label} · not runnable"), RED),
                };
                ui.label(RichText::new(text).size(10.5).color(color))
                    .on_hover_text(harness.describe(lang));
            });
        });

        attempted
    }

    /// The strip above the diff: what changed, and what to do about it.
    fn fix_banner(&mut self, ui: &mut Ui, key: &(String, String)) {
        let Some(review) = &self.review else { return };
        let (removed, added) = (review.diff.removed, review.diff.added);
        let (groups, taken) = (review.diff.hunks, review.accepted_count());
        let empty = review.diff.is_empty();

        let mut apply = false;
        let mut discard = false;
        let mut all = None;

        egui::Frame::default()
            .fill(tint(ACCENT))
            .stroke(egui::Stroke::new(1.0, ACCENT))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if empty {
                        ui.label(
                            RichText::new(
                                "💬 the AI changed nothing — your code already matches it.",
                            )
                            .size(12.0)
                            .color(TEXT),
                        );
                    } else {
                        ui.label(
                            RichText::new("💬 the AI's fix, against what you wrote:")
                                .size(12.0)
                                .color(TEXT),
                        );
                        pill(ui, &format!("−{removed}"), RED, tint(RED));
                        pill(ui, &format!("+{added}"), GREEN, tint(GREEN));
                        ui.label(
                            RichText::new(format!(
                                "· {taken} of {groups} change{} kept",
                                if groups == 1 { "" } else { "s" }
                            ))
                            .size(11.5)
                            .color(TEXT_DIM),
                        );
                        if groups > 1 {
                            if mini_btn(ui, "keep all").clicked() {
                                all = Some(true);
                            }
                            if mini_btn(ui, "keep none").clicked() {
                                all = Some(false);
                            }
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if toggle_btn(ui, if empty { "✔ close" } else { "✔ apply" }, true)
                            .on_hover_text("Put the ticked changes into your code")
                            .clicked()
                        {
                            apply = true;
                        }
                        if mini_btn(ui, "✖ discard")
                            .on_hover_text("Leave your code exactly as it is")
                            .clicked()
                        {
                            discard = true;
                        }
                    });
                });
            });

        if let Some(on) = all {
            if let Some(review) = &mut self.review {
                review.accepted.fill(on);
            }
        }
        if apply {
            if let Some(review) = self.review.take() {
                let merged = review.diff.apply(&review.accepted);
                self.solutions.insert(key.clone(), merged);
            }
        } else if discard {
            // The editor was never touched, so there is nothing to restore.
            self.review = None;
        }
    }

    fn io_panel(&mut self, ui: &mut Ui, pack: &dsa_content::ProblemPack, lang_label: &str) {
        side_head(ui, "STDIN");
        ui.add(
            egui::TextEdit::multiline(&mut self.stdin)
                .code_editor()
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );

        side_head(ui, "OUTPUT");
        let avail = ui.available_height();
        ScrollArea::vertical()
            .id_salt("out")
            .max_height(avail * 0.42)
            .auto_shrink([false, false])
            .show(ui, |ui| match (&self.result, self.running) {
                (_, true) => side_empty(ui, "⏳ compiling & running…"),
                (None, false) => side_empty(
                    ui,
                    &format!("press ▶ Run — code executes on a real {lang_label} toolchain"),
                ),
                (Some(r), false) => {
                    if let Some(e) = &r.error {
                        code_block(ui, e, RED);
                    }
                    if !r.compile_output.is_empty() {
                        pill(
                            ui,
                            if r.compiled {
                                "build output"
                            } else {
                                "compile error"
                            },
                            if r.compiled { TEXT_DIM } else { RED },
                            tint(if r.compiled { TEXT_DIM } else { RED }),
                        );
                        code_block(
                            ui,
                            &r.compile_output,
                            if r.compiled { TEXT_DIM } else { RED },
                        );
                    }
                    if r.compiled {
                        if !r.stdout.trim().is_empty() {
                            code_block(ui, r.stdout.trim_end(), TEXT);
                        }
                        if !r.stderr.trim().is_empty() {
                            pill(ui, "stderr", RED, tint(RED));
                            code_block(ui, r.stderr.trim_end(), RED);
                        }
                        let ok = r.exit_code == Some(0);
                        ui.horizontal(|ui| {
                            pill(
                                ui,
                                &format!(
                                    "exit code {}",
                                    r.exit_code.map(|c| c.to_string()).unwrap_or("—".into())
                                ),
                                if ok { GREEN } else { RED },
                                tint(if ok { GREEN } else { RED }),
                            );
                            ui.label(
                                RichText::new(format!("{:.1}s", r.duration.as_secs_f32()))
                                    .size(10.5)
                                    .color(TEXT_DIM),
                            );
                        });
                    }
                }
            });

        side_head(ui, "TESTS");
        ScrollArea::vertical()
            .id_salt("tests")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, case) in pack.meta.tests.iter().enumerate() {
                    let outcome = self.tests.get(i);
                    let (badge, color) = match outcome.map(|o| o.status) {
                        Some(TestStatus::Passed) => ("✔", GREEN),
                        Some(TestStatus::Failed) => ("✖", RED),
                        Some(TestStatus::CompileError)
                        | Some(TestStatus::Crashed)
                        | Some(TestStatus::TimedOut)
                        | Some(TestStatus::HarnessError) => ("!", AMBER),
                        None => ("·", TEXT_DIM),
                    };
                    let stdin = serialize_input(&pack.meta.inputs, &case.input);
                    egui::Frame::default()
                        .fill(PANEL2)
                        .stroke(egui::Stroke::new(
                            1.0,
                            if outcome.is_some() { color } else { BORDER },
                        ))
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(7))
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.label(RichText::new(badge).size(12.0).color(color));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "in: {}",
                                            stdin.trim().replace('\n', " | ")
                                        ))
                                        .monospace()
                                        .size(11.0)
                                        .color(TEXT_DIM),
                                    );
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.label(RichText::new("want:").size(11.0).color(TEXT_DIM));
                                        ui.label(
                                            RichText::new(&case.expected)
                                                .monospace()
                                                .size(11.0)
                                                .color(TEXT),
                                        );
                                        if let Some(o) = outcome {
                                            if o.status == TestStatus::Failed {
                                                ui.label(
                                                    RichText::new("· got:")
                                                        .size(11.0)
                                                        .color(TEXT_DIM),
                                                );
                                                ui.label(
                                                    RichText::new(if o.actual.is_empty() {
                                                        "(nothing)"
                                                    } else {
                                                        &o.actual
                                                    })
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(RED),
                                                );
                                            }
                                        }
                                    });
                                    if let Some(o) = outcome {
                                        if !o.detail.trim().is_empty() {
                                            ui.label(
                                                RichText::new(truncate(&o.detail, 400))
                                                    .monospace()
                                                    .size(10.0)
                                                    .color(AMBER),
                                            );
                                        }
                                    }
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });
    }

    // ── actions ─────────────────────────────────────────────────────────────

    /// The solution buffer wrapped in the pack's runnable harness.
    fn assembled(
        &self,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
        lang: &str,
    ) -> String {
        let solution = self.solutions.get(key).cloned().unwrap_or_default();
        assemble(&solution, pack, lang)
    }

    /// What Run and the tests compile.
    ///
    /// A full program the user has taken over is theirs and is used verbatim.
    /// Otherwise the solution is assembled fresh — so pressing Run from either
    /// view always compiles what is actually on screen.
    fn program(
        &self,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
        lang: &str,
    ) -> String {
        if self.view == View::Full && self.full_edited.contains(key) {
            return self.full.get(key).cloned().unwrap_or_default();
        }
        self.assembled(key, pack, lang)
    }

    fn switch_view(
        &mut self,
        view: View,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
        lang: &str,
    ) {
        if self.review.is_some() {
            return; // an in-flight fix review owns the editor
        }
        // Rebuild on the way in unless the user owns this buffer, or the view
        // would show the program their solution looked like several edits ago.
        if view == View::Full && !self.full_edited.contains(key) {
            let assembled = self.assembled(key, pack, lang);
            self.full.insert(key.clone(), assembled);
        }
        self.view = view;
    }

    fn reset_code(&mut self, key: &(String, String), pack: &dsa_content::ProblemPack, lang: &str) {
        match self.view {
            View::Full => {
                let assembled = self.assembled(key, pack, lang);
                self.full.insert(key.clone(), assembled);
                self.full_edited.remove(key);
            }
            View::Solution => {
                self.review = None;
                self.solutions.insert(key.clone(), starter_for(pack, lang));
            }
        }
        self.result = None;
    }

    fn start_fix_review(&mut self, key: &(String, String), fixed: String) {
        self.view = View::Solution;
        let original = self.solutions.get(key).cloned().unwrap_or_default();
        let diff = dsa_core::diff::diff(&original, &fixed);
        let accepted = vec![true; diff.hunks];
        self.review = Some(Review {
            key: key.clone(),
            diff,
            accepted,
        });
    }

    fn spawn_run(
        &mut self,
        harness: &Harness,
        lang: &str,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
    ) {
        let source = self.program(key, pack, lang);
        let stdin = self.stdin.clone();
        let langs = harness
            .language(lang)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        let lang = lang.to_string();
        let (tx, rx) = channel();
        self.pending = Some(rx);
        self.running = true;
        self.result = None;
        std::thread::spawn(move || {
            let h = Harness::detect(&langs);
            let _ = tx.send(Job::Run(h.run(&lang, &source, &stdin)));
        });
    }

    fn spawn_tests(
        &mut self,
        harness: &Harness,
        lang: &str,
        key: &(String, String),
        pack: &dsa_content::ProblemPack,
    ) {
        let source = self.program(key, pack, lang);
        let fields = pack.meta.inputs.clone();
        let cases = pack.meta.tests.clone();
        let langs = harness
            .language(lang)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        let lang = lang.to_string();
        let (tx, rx) = channel();
        self.pending = Some(rx);
        self.running = true;
        self.tests.clear();
        std::thread::spawn(move || {
            let h = Harness::detect(&langs);
            let _ = tx.send(Job::Tests(run_tests(&h, &lang, &source, &fields, &cases)));
        });
    }
}

// ── pieces ──────────────────────────────────────────────────────────────────

/// Assemble what the AI needs to know: the problem, the category's technique
/// vocabulary from the helper, and a couple of worked examples. Giving the
/// model the same context a human tutor would have is most of what separates a
/// useful hint from a generic one.
fn brief_for(lib: &Library, pack: &dsa_content::ProblemPack) -> Brief {
    let m = &pack.meta;
    let topics = lib
        .guide
        .for_category(&m.category)
        .iter()
        .map(|t| t.title.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    // Two examples is enough to pin down the output format without eating the
    // context window.
    let examples = m
        .tests
        .iter()
        .take(2)
        .map(|t| {
            let ins = m
                .inputs
                .iter()
                .map(|f| {
                    let v = t.input.get(&f.name).map(fmt_value).unwrap_or_default();
                    format!("{} = {}", f.name, v)
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("Input: {ins} → Output: {}", t.expected)
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    Brief {
        title: m.title.clone(),
        difficulty: match m.difficulty {
            dsa_core::problem::Difficulty::Easy => "Easy",
            dsa_core::problem::Difficulty::Medium => "Medium",
            dsa_core::problem::Difficulty::Hard => "Hard",
        }
        .into(),
        description: m.description.clone(),
        approach: m.approach.clone(),
        complexity: m.complexity.clone(),
        category: m.category.clone(),
        topics,
        examples,
    }
}

/// LeetCode-style statement beside the editor.
fn question_panel(ui: &mut Ui, pack: &dsa_content::ProblemPack) {
    ScrollArea::vertical().id_salt("q").auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&pack.meta.title).size(18.0).strong().color(TEXT));
            diff_pill(ui, pack.meta.difficulty);
        });
        ui.label(RichText::new(&pack.meta.category).size(11.5).color(TEXT_DIM));
        ui.add_space(10.0);
        // The statement is the one thing on this screen that gets read as
        // prose rather than scanned, so it is the one thing set at full
        // contrast and a comfortable size.
        ui.label(
            RichText::new(&pack.meta.description)
                .size(13.5)
                .color(TEXT)
                .line_height(Some(19.0)),
        );
        ui.add_space(4.0);
        if !pack.meta.complexity.trim().is_empty() {
            ui.label(
                RichText::new(&pack.meta.complexity)
                    .size(11.5)
                    .monospace()
                    .color(ACCENT2),
            );
        }
        ui.add_space(10.0);

        for (i, test) in pack.meta.tests.iter().enumerate() {
            side_head(ui, &format!("EXAMPLE {}", i + 1));
            let mut lines: Vec<String> = pack
                .meta
                .inputs
                .iter()
                .map(|f| {
                    let v = test.input.get(&f.name).map(fmt_value).unwrap_or_default();
                    format!("Input: {} = {}", f.name, v)
                })
                .collect();
            lines.push(format!("Output: {}", test.expected));
            code_block(ui, &lines.join("\n"), TEXT);
            ui.add_space(6.0);
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "💡 switch to ⏵ Visualize for the animated step-by-step walkthrough of this problem.",
            )
            .size(11.5)
            .color(TEXT_DIM),
        );
    });
}

fn fmt_value(v: &dsa_core::problem::InputValue) -> String {
    use dsa_core::problem::InputValue as V;
    match v {
        V::List(items) => {
            format!(
                "[{}]",
                items.iter().map(fmt_value).collect::<Vec<_>>().join(",")
            )
        }
        V::Str(s) => format!("\"{s}\""),
        other => other.to_editable(),
    }
}

/// The AI's fix, line by line, against what the user wrote.
///
/// Two gutters, as a code host shows them: the left number is the line in your
/// code, the right is the line in the proposal, and a line that exists on only
/// one side has only one number. Removals are red and dimmed — they are on
/// their way out; additions are green and at full contrast.
///
/// Each change group carries its own tick, so a fix that corrects one thing and
/// rewrites another can be taken in part.
fn diff_view(ui: &mut Ui, review: &mut Review) {
    let row = ui.text_style_height(&egui::TextStyle::Monospace);

    egui::Frame::default()
        .fill(alpha(PANEL, 0xcc))
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            if review.diff.is_empty() {
                side_empty(
                    ui,
                    "Nothing to change — the model's version matches yours line for line.",
                );
                return;
            }

            ScrollArea::both()
                .id_salt("fix-diff")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut drawn_hunk: Option<usize> = None;

                    for r in review.diff.rows(3) {
                        let i = match r {
                            Row::Folded(n) => {
                                folded_marker(ui, n, row);
                                continue;
                            }
                            Row::Line(i) => i,
                        };
                        let line = &review.diff.lines[i];

                        // One header per group, above its first line. A group is
                        // an unbroken run, so this fires exactly once for each.
                        if let Some(h) = line.hunk {
                            if drawn_hunk != Some(h) {
                                drawn_hunk = Some(h);
                                let n = review.diff.hunks;
                                hunk_header(ui, h, n, &mut review.accepted);
                            }
                        }

                        let taken = line
                            .hunk
                            .is_none_or(|h| review.accepted.get(h).copied().unwrap_or(true));
                        diff_row(ui, line, row, taken);
                    }
                });
        });
}

fn diff_row(ui: &mut Ui, line: &dsa_core::diff::Line, row: f32, taken: bool) {
    let (bg, marker, fg) = match line.change {
        Change::Same => (egui::Color32::TRANSPARENT, " ", alpha(TEXT, 0xcc)),
        Change::Removed => (alpha(RED, 0x1c), "−", alpha(TEXT, 0xaa)),
        Change::Added => (alpha(GREEN, 0x1c), "+", TEXT),
    };
    // A group that has been un-ticked is not going to happen, so it stops
    // shouting — still legible, no longer a claim about the result.
    let fade = if taken { 1.0 } else { 0.42 };

    let full = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(full, row), egui::Sense::hover());
    if bg != egui::Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::ZERO, bg.gamma_multiply(fade));
    }

    let p = ui.painter();
    let mono = |size: f32| egui::FontId::monospace(size);
    let num = |n: Option<usize>| n.map(|v| v.to_string()).unwrap_or_default();
    p.text(
        egui::pos2(rect.left() + 34.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        num(line.old_no),
        mono(10.5),
        alpha(TEXT_DIM, 0xaa),
    );
    p.text(
        egui::pos2(rect.left() + 70.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        num(line.new_no),
        mono(10.5),
        alpha(TEXT_DIM, 0xaa),
    );
    p.text(
        egui::pos2(rect.left() + 84.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        marker,
        mono(12.0),
        match line.change {
            Change::Removed => RED.gamma_multiply(fade),
            Change::Added => GREEN.gamma_multiply(fade),
            Change::Same => TEXT_DIM,
        },
    );
    p.text(
        egui::pos2(rect.left() + 96.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &line.text,
        mono(12.0),
        fg.gamma_multiply(fade),
    );
}

/// The tick that decides whether one change group makes it into the code.
fn hunk_header(ui: &mut Ui, index: usize, total: usize, accepted: &mut [bool]) {
    let on = accepted.get(index).copied().unwrap_or(true);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        if toggle_btn(
            ui,
            &format!(
                "{} change {} of {total}",
                if on { "✔" } else { "☆" },
                index + 1
            ),
            on,
        )
        .on_hover_text(if on {
            "This change will be applied — click to leave your version"
        } else {
            "Your version is kept — click to take the AI's"
        })
        .clicked()
        {
            if let Some(slot) = accepted.get_mut(index) {
                *slot = !on;
            }
        }
    });
    ui.add_space(3.0);
}

/// "… 12 unchanged lines", standing in for the context nobody needs to read.
fn folded_marker(ui: &mut Ui, n: usize, row: f32) {
    let full = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(full, row), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 96.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("⋯ {n} unchanged line{}", if n == 1 { "" } else { "s" }),
        egui::FontId::monospace(10.5),
        alpha(TEXT_DIM, 0x99),
    );
}

/// Code editor with a line-number gutter, kept aligned by sharing the
/// monospace row height. Returns the text area's response, so the caller can
/// tell an edit from a mere repaint.
fn editor(ui: &mut Ui, buffer: &mut String, syntax: &str) -> egui::Response {
    let row = ui.text_style_height(&egui::TextStyle::Monospace);
    let lines = buffer.lines().count().max(1);
    let _ = syntax;

    // The editor gets its own surface. Without it the code floats on the page
    // background and the (usually large) empty area below it reads as a hole
    // rather than as the rest of the document.
    egui::Frame::default()
        .fill(alpha(PANEL, 0xcc))
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ScrollArea::both()
                .id_salt("editor")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| -> egui::Response {
                        ui.vertical(|ui| {
                            ui.add_space(2.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for n in 1..=lines {
                                ui.allocate_ui(egui::vec2(30.0, row), |ui| {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(n.to_string())
                                                .monospace()
                                                .size(11.0)
                                                .color(TEXT_DIM),
                                        );
                                    });
                                });
                            }
                        });
                        ui.add(
                            egui::TextEdit::multiline(buffer)
                                .code_editor()
                                .frame(false)
                                .desired_width(ui.available_width())
                                .desired_rows(lines.max(24)),
                        )
                    })
                    .inner
                })
                .inner
        })
        .inner
}

/// What the editor opens on: the pack's reference solution with every function
/// body replaced by "write your code here".
///
/// A pack with no source for this language has nothing to blank, so it falls
/// back to the harness — better a full program to edit than an empty editor.
fn starter_for(pack: &dsa_content::ProblemPack, lang: &str) -> String {
    match pack.source(lang) {
        Some(parsed) if !parsed.clean.trim().is_empty() => {
            dsa_core::practice::starter(&parsed.clean, lang)
        }
        _ => pack.practice.get(lang).cloned().unwrap_or_default(),
    }
}

/// The program the solution gets wrapped in: the pack's hand-written harness if
/// it ships one, otherwise one derived from its input schema.
///
/// `None` means the problem cannot be run from the solution view at all — the
/// schema and the entry signature do not line up, and guessing would be worse
/// than saying so.
fn harness_for(pack: &dsa_content::ProblemPack, lang: &str) -> Option<String> {
    if let Some(shipped) = pack.practice.get(lang) {
        return Some(shipped.clone());
    }
    let reference = pack.source(lang)?;
    dsa_core::synth::synthesize(lang, &pack.meta, &reference.clean)
}

/// The user's solution spliced into that harness. With no harness their text is
/// the whole program, which is what the "full program" view is for.
fn assemble(solution: &str, pack: &dsa_content::ProblemPack, lang: &str) -> String {
    let reference = pack.source(lang).map(|p| p.clean.as_str()).unwrap_or("");
    dsa_core::practice::assemble(solution, reference, harness_for(pack, lang).as_deref())
}

fn log_hint(lang: &str) -> &'static str {
    match lang {
        "cpp" => "cout << \"dbg: \" << x << endl;",
        "java" => "System.out.println(\"dbg: \" + x);",
        "python" => "print(\"dbg:\", x)",
        _ => "fmt.Println(\"dbg:\", x)",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::InputValue;

    #[test]
    fn examples_render_leetcode_style_values() {
        assert_eq!(
            fmt_value(&InputValue::List(vec![
                InputValue::Int(1),
                InputValue::Int(2)
            ])),
            "[1,2]"
        );
        assert_eq!(fmt_value(&InputValue::Str("ab".into())), "\"ab\"");
        assert_eq!(fmt_value(&InputValue::Int(9)), "9");
    }

    #[test]
    fn the_log_hint_matches_the_language() {
        assert!(log_hint("go").contains("fmt.Println"));
        assert!(log_hint("cpp").contains("cout"));
        assert!(log_hint("java").contains("System.out"));
        assert!(log_hint("python").contains("print"));
    }

    #[test]
    fn a_fix_under_review_does_not_touch_the_editor() {
        // The whole reason the diff exists: the code you wrote stays on screen
        // and unchanged until you say otherwise, so you can see your mistake.
        let mut p = Practice::default();
        let key = ("two-sum".to_string(), "go".to_string());
        p.solutions.insert(key.clone(), "mine\n".into());

        p.start_fix_review(&key, "theirs\n".into());
        assert!(p.review.is_some());
        assert_eq!(p.solutions[&key], "mine\n", "the buffer was overwritten");

        // Discarding is therefore free — there is nothing to put back.
        p.review = None;
        assert_eq!(p.solutions[&key], "mine\n");
    }

    #[test]
    fn the_review_diffs_the_proposal_against_what_the_user_wrote() {
        let mut p = Practice::default();
        let key = ("two-sum".to_string(), "go".to_string());
        p.solutions
            .insert(key.clone(), "func f() int {\n    return 1\n}\n".into());
        p.start_fix_review(&key, "func f() int {\n    return 2\n}\n".into());

        let review = p.review.as_ref().expect("review is open");
        assert_eq!((review.diff.removed, review.diff.added), (1, 1));
        assert_eq!(review.diff.hunks, 1);
        assert_eq!(review.accepted, vec![true], "changes arrive ticked");
        assert_eq!(review.accepted_count(), 1);
    }

    #[test]
    fn applying_a_review_writes_only_the_ticked_changes() {
        let mut p = Practice::default();
        let key = ("two-sum".to_string(), "go".to_string());
        let mine = "a\nWRONG1\nb\nc\nd\ne\nf\nWRONG2\ng\n";
        p.solutions.insert(key.clone(), mine.into());
        p.start_fix_review(&key, "a\nRIGHT1\nb\nc\nd\ne\nf\nRIGHT2\ng\n".into());

        // Take the first correction, keep my own version of the second.
        let review = p.review.as_mut().unwrap();
        assert_eq!(review.diff.hunks, 2);
        review.accepted[1] = false;

        let review = p.review.take().unwrap();
        let merged = review.diff.apply(&review.accepted);
        p.solutions.insert(key.clone(), merged);

        assert!(p.solutions[&key].contains("RIGHT1"));
        assert!(p.solutions[&key].contains("WRONG2"));
    }

    #[test]
    fn switching_language_drops_a_review_meant_for_the_other_one() {
        // Applying it would splice a Go fix into the C++ buffer.
        let mut p = Practice::default();
        let go = ("two-sum".to_string(), "go".to_string());
        p.solutions.insert(go.clone(), "mine\n".into());
        p.start_fix_review(&go, "theirs\n".into());

        let cpp = ("two-sum".to_string(), "cpp".to_string());
        assert!(p.review.as_ref().is_some_and(|r| r.key != cpp));
    }

    #[test]
    fn run_context_summarises_failures_for_the_ai() {
        let mut p = Practice {
            result: None,
            ..Default::default()
        };
        p.result = Some(RunOutcome {
            compiled: true,
            exit_code: Some(1),
            stderr: "index out of range".into(),
            ..Default::default()
        });
        p.tests.push(TestOutcome {
            name: "case 1".into(),
            status: TestStatus::Failed,
            stdin: "2 7\n9\n".into(),
            expected: "0 1".into(),
            actual: "1 0".into(),
            detail: String::new(),
            duration: std::time::Duration::ZERO,
        });
        let ctx = p.run_context(&[]);
        assert!(ctx.contains("index out of range"));
        assert!(ctx.contains("test 1 FAILED"));
        assert!(ctx.contains("expected \"0 1\""));
        assert!(ctx.contains("got \"1 0\""));
    }

    #[test]
    fn a_passing_test_adds_no_noise_to_the_ai_context() {
        let mut p = Practice::default();
        p.tests.push(TestOutcome {
            name: "case 1".into(),
            status: TestStatus::Passed,
            stdin: String::new(),
            expected: "1".into(),
            actual: "1".into(),
            detail: String::new(),
            duration: std::time::Duration::ZERO,
        });
        assert_eq!(p.run_context(&[]), "");
    }

    fn outcome(status: TestStatus) -> TestOutcome {
        TestOutcome {
            name: "case".into(),
            status,
            stdin: String::new(),
            expected: "1".into(),
            actual: "1".into(),
            detail: String::new(),
            duration: std::time::Duration::ZERO,
        }
    }

    #[test]
    fn a_problem_is_only_ticked_off_when_every_case_passed() {
        let pass = outcome(TestStatus::Passed);
        assert!(all_tests_passed(&[pass.clone(), pass.clone()], 2));

        // One of three is not a solution.
        assert!(!all_tests_passed(
            &[pass.clone(), outcome(TestStatus::Failed), pass.clone()],
            3
        ));
        // Nor is a sweep that stopped early — the rest were never run.
        assert!(!all_tests_passed(std::slice::from_ref(&pass), 3));
        // Nor a compile error dressed up as a short run.
        assert!(!all_tests_passed(&[outcome(TestStatus::CompileError)], 1));
        // A problem with no test cases can never auto-solve itself.
        assert!(!all_tests_passed(&[], 0));
        assert!(!all_tests_passed(&[pass], 0));
    }

    #[test]
    fn long_output_is_truncated_for_the_prompt() {
        let long = "x".repeat(2000);
        assert!(truncate(&long, 100).chars().count() <= 101);
        assert_eq!(truncate("short", 100), "short");
    }
}
