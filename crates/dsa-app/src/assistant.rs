//! The 💬 AI assist panel.
//!
//! Ported from the web version, including the three modes and their rules:
//! Interview never hands over code, Guide answers exactly what was asked and
//! offers clickable choices when the question is vague, and Fix reviews the
//! current solution and shows its correction as a diff against what you wrote.
//!
//! Everything talks to a local Ollama server. Streaming happens on a worker
//! thread; this module only polls it, so the window keeps repainting while a
//! model thinks.

use crate::settings::Settings;
use crate::style::*;
use dsa_ai::{
    attach_code, chat_stream, extract_last_code_block, extract_think, list_models, match_indent,
    parse_options, segments, Channel, ChatMessage, ChatOptions, Event, Mode, PromptContext,
    Prompts, Stream, HISTORY_TURNS,
};
use egui::{Align, Layout, RichText, ScrollArea, Ui};
use std::sync::mpsc::{channel, Receiver};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Checking,
    Ready,
    Offline,
}

#[derive(Clone, Default)]
struct UiMsg {
    from_user: bool,
    content: String,
    thinking: String,
    code_attached: bool,
}

#[derive(Default)]
struct FixState {
    streaming: String,
    thinking: String,
    analysis: String,
    proposed: bool,
    no_changes: bool,
}

/// Where the tokens currently arriving should be appended.
enum Target {
    Chat(Mode),
    Fix,
}

/// Everything the prompts need about the current problem, prepared by the
/// caller (which is the only place with access to the guide and the tests).
#[derive(Clone, Default)]
pub struct Brief {
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub approach: String,
    pub complexity: String,
    pub category: String,
    /// Technique names the helper associates with this category.
    pub topics: String,
    /// One or two worked examples, `Input … → Output …`.
    pub examples: String,
}

impl Brief {
    fn context<'a>(&'a self, lang_id: &'a str, lang_label: &'a str) -> PromptContext<'a> {
        PromptContext {
            title: &self.title,
            difficulty: &self.difficulty,
            description: &self.description,
            approach: &self.approach,
            complexity: &self.complexity,
            category: &self.category,
            topics: &self.topics,
            examples: &self.examples,
            lang_label,
            lang_id,
        }
    }
}

/// What the assistant wants the practice tab to do.
pub enum AiAction {
    None,
    /// Open this corrected solution in the editor as a reviewable change.
    ProposeFix(String),
}

pub struct Assistant {
    mode: Mode,
    status: Status,
    error: String,
    models: Vec<String>,
    show_settings: bool,

    interview: Vec<UiMsg>,
    guide: Vec<UiMsg>,
    input: String,
    attach: bool,
    fix: Option<FixState>,

    stream: Option<Stream>,
    target: Option<Target>,
    /// Snapshot of the code the running fix was asked about.
    fix_snapshot: String,
    models_rx: Option<Receiver<Result<Vec<String>, String>>>,
    prompts: Prompts,
}

impl Default for Assistant {
    fn default() -> Self {
        Self {
            mode: Mode::Interview,
            status: Status::Checking,
            error: String::new(),
            models: Vec::new(),
            show_settings: false,
            interview: Vec::new(),
            guide: Vec::new(),
            input: String::new(),
            attach: true,
            fix: None,
            stream: None,
            target: None,
            fix_snapshot: String::new(),
            models_rx: None,
            prompts: Prompts::default(),
        }
    }
}

impl Assistant {
    pub fn set_prompts(&mut self, prompts: Prompts) {
        self.prompts = prompts;
    }

    pub fn busy(&self) -> bool {
        self.stream.is_some()
    }

    /// Kick off a model list refresh on a worker thread.
    pub fn connect(&mut self, url: &str) {
        self.status = Status::Checking;
        self.error.clear();
        let (tx, rx) = channel();
        let url = url.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(list_models(&url));
        });
        self.models_rx = Some(rx);
    }

    /// Drain the worker threads. Called once per frame; returns true when
    /// something changed and the UI should repaint.
    pub fn poll(&mut self, settings: &mut Settings) -> bool {
        let mut dirty = false;

        if let Some(rx) = &self.models_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(names) => {
                        self.status = Status::Ready;
                        // Keep the saved model when the server still has it.
                        if settings.ollama_model.is_empty()
                            || !names.contains(&settings.ollama_model)
                        {
                            settings.ollama_model = names.first().cloned().unwrap_or_default();
                        }
                        self.models = names;
                    }
                    Err(e) => {
                        self.status = Status::Offline;
                        self.error = e;
                        self.models.clear();
                    }
                }
                self.models_rx = None;
                dirty = true;
            }
        }

        if let Some(stream) = &mut self.stream {
            let events = stream.poll();
            if !events.is_empty() {
                dirty = true;
            }
            let mut finished = false;
            for ev in events {
                match ev {
                    Event::Token(channel, text) => self.append(channel, &text),
                    Event::Error(msg) => {
                        self.append(Channel::Content, &format!("\n⚠ {msg}"));
                        finished = true;
                    }
                    Event::Done => finished = true,
                }
            }
            if finished {
                self.stream = None;
                return true;
            }
        }
        dirty
    }

    fn append(&mut self, channel: Channel, text: &str) {
        match self.target {
            Some(Target::Fix) => {
                let fix = self.fix.get_or_insert_with(FixState::default);
                match channel {
                    Channel::Content => fix.streaming.push_str(text),
                    Channel::Thinking => fix.thinking.push_str(text),
                }
            }
            Some(Target::Chat(mode)) => {
                let msgs = if mode == Mode::Interview {
                    &mut self.interview
                } else {
                    &mut self.guide
                };
                if let Some(last) = msgs.last_mut() {
                    match channel {
                        Channel::Content => last.content.push_str(text),
                        Channel::Thinking => last.thinking.push_str(text),
                    }
                }
            }
            None => {}
        }
    }

    /// Called when a fix finishes streaming: pull the corrected function out.
    fn finish_fix(&mut self) -> AiAction {
        let Some(fix) = self.fix.as_mut() else {
            return AiAction::None;
        };
        let (_, body) = extract_think(&fix.streaming);
        let (code, rest) = extract_last_code_block(&body);
        let analysis = rest.trim_start_matches("ISSUES:").trim().to_string();
        fix.streaming.clear();

        match code {
            None => {
                fix.analysis = if analysis.is_empty() {
                    "The model returned nothing usable — try again or pick a bigger model.".into()
                } else {
                    analysis
                };
                AiAction::None
            }
            Some(code) => {
                let aligned = match_indent(&code, &self.fix_snapshot);
                fix.analysis = analysis;
                if aligned.trim() == self.fix_snapshot.trim() {
                    fix.no_changes = true;
                    AiAction::None
                } else {
                    fix.proposed = true;
                    AiAction::ProposeFix(aligned)
                }
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(s) = &self.stream {
            s.cancel();
        }
        self.stream = None;
        self.target = None;
    }

    // ── rendering ───────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        brief: &Brief,
        lang_id: &str,
        lang_label: &str,
        code: &str,
        run_context: &str,
    ) -> AiAction {
        let mut action = AiAction::None;

        // A finished fix stream needs parsing exactly once.
        if self.stream.is_none() && matches!(self.target, Some(Target::Fix)) {
            self.target = None;
            action = self.finish_fix();
        }
        if self.stream.is_none() {
            self.target = None;
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("💬 AI assist · local")
                    .size(11.0)
                    .color(TEXT_DIM),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (text, color) = match self.status {
                    Status::Checking => ("…", TEXT_DIM),
                    Status::Ready => ("● online", GREEN),
                    Status::Offline => ("○ offline", TEXT_DIM),
                };
                ui.label(RichText::new(text).size(11.0).color(color));
            });
        });

        self.config_row(ui, settings);

        if self.status == Status::Offline {
            ui.add_space(4.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "Can't reach Ollama at {}. Start it with `ollama serve` and pull a model, \
                         e.g. `ollama pull gemma3:4b`.",
                        settings.ollama_url
                    ))
                    .size(11.5)
                    .color(TEXT_DIM),
                );
                if !self.error.is_empty() {
                    ui.label(RichText::new(&self.error).size(10.5).color(RED));
                }
            });
        }

        ui.add_space(6.0);
        let mut mode = self.mode;
        if seg(
            ui,
            &mut mode,
            &[
                (Mode::Interview, "🎤 Interview"),
                (Mode::Guide, "🗺 Guide"),
                (Mode::Fix, "🔧 Fix"),
            ],
        ) {
            self.mode = mode;
        }
        ui.add_space(6.0);

        if self.mode == Mode::Fix {
            self.fix_ui(ui, settings, brief, lang_id, lang_label, code, run_context);
        } else {
            self.chat_ui(ui, settings, brief, lang_id, lang_label, code);
        }

        action
    }

    fn config_row(&mut self, ui: &mut Ui, settings: &mut Settings) {
        ui.horizontal(|ui| {
            let current = if settings.ollama_model.is_empty() {
                "no models".to_string()
            } else {
                settings.ollama_model.clone()
            };
            egui::ComboBox::from_id_salt("ai-model")
                .selected_text(RichText::new(current).size(11.5))
                .width(ui.available_width() - 70.0)
                .show_ui(ui, |ui| {
                    for name in self.models.clone() {
                        ui.selectable_value(
                            &mut settings.ollama_model,
                            name.clone(),
                            RichText::new(name).size(11.5),
                        );
                    }
                });
            if mini_btn(ui, "⟳").on_hover_text("Reconnect").clicked() {
                let url = settings.ollama_url.clone();
                self.connect(&url);
            }
            if mini_btn(ui, "⚙")
                .on_hover_text("Ollama server URL")
                .clicked()
            {
                self.show_settings = !self.show_settings;
            }
        });

        if self.show_settings {
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut settings.ollama_url)
                        .hint_text(dsa_ai::DEFAULT_OLLAMA_URL)
                        .desired_width(ui.available_width()),
                );
                if resp.lost_focus() {
                    let url = settings.ollama_url.trim_end_matches('/').to_string();
                    settings.ollama_url = url.clone();
                    self.connect(&url);
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn chat_ui(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        brief: &Brief,
        lang_id: &str,
        lang_label: &str,
        code: &str,
    ) {
        let mode = self.mode;
        let msgs = if mode == Mode::Interview {
            self.interview.clone()
        } else {
            self.guide.clone()
        };
        let busy = self.busy();
        let mut send: Option<String> = None;

        let bottom = 96.0;
        ScrollArea::vertical()
            .id_salt("ai-chat")
            .stick_to_bottom(true)
            .max_height((ui.available_height() - bottom).max(120.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if msgs.is_empty() {
                    side_empty(ui, mode.empty_hint());
                }
                for (i, m) in msgs.iter().enumerate() {
                    let is_last = i + 1 == msgs.len();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(if m.from_user { "you" } else { mode.speaker() })
                            .size(10.5)
                            .color(if m.from_user { ACCENT } else { ACCENT2 }),
                    );

                    let (inline_think, body) = extract_think(&m.content);
                    let thinking = format!("{}{}", m.thinking, inline_think);
                    if !thinking.trim().is_empty() {
                        egui::CollapsingHeader::new(
                            RichText::new("💭 thinking").size(11.0).color(TEXT_DIM),
                        )
                        .id_salt(format!("think-{i}"))
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label(RichText::new(thinking.trim()).size(11.0).color(TEXT_DIM));
                        });
                    }

                    // Guide mode turns OPTION: lines into buttons.
                    let (text, options) = if !m.from_user && mode == Mode::Guide {
                        parse_options(&body)
                    } else {
                        (body.clone(), Vec::new())
                    };

                    rich_text(ui, &text, mode == Mode::Interview && !m.from_user, i);

                    if busy && is_last && !m.from_user {
                        ui.label(RichText::new("▍").size(12.0).color(ACCENT));
                    }
                    if m.code_attached && m.from_user {
                        pill(ui, "📎 code attached", TEXT_DIM, PANEL2);
                    }
                    if is_last && !busy {
                        for opt in options {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(&opt).size(12.0).color(TEXT))
                                        .fill(PANEL2)
                                        .stroke(egui::Stroke::new(1.0, ACCENT))
                                        .corner_radius(egui::CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                send = Some(opt.clone());
                            }
                            ui.add_space(3.0);
                        }
                    }
                }
            });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.attach,
                RichText::new("📎 attach my code").size(11.5),
            );
            if busy && mini_btn(ui, "■ stop").clicked() {
                self.stop();
            }
        });

        let offline = self.status == Status::Offline;
        ui.horizontal(|ui| {
            let hint = if offline {
                "Ollama offline…"
            } else {
                mode.placeholder()
            };
            let resp = ui.add_enabled(
                !offline && !busy,
                egui::TextEdit::multiline(&mut self.input)
                    .hint_text(hint)
                    .desired_rows(2)
                    .desired_width(ui.available_width() - 44.0),
            );
            // Enter sends, Shift+Enter makes a newline.
            if resp.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
            {
                send = Some(self.input.trim().to_string());
            }
            let can_send = !offline && !busy && !self.input.trim().is_empty();
            if apply_btn(ui, "➤", can_send).clicked() {
                send = Some(self.input.trim().to_string());
            }
        });

        if let Some(text) = send {
            let text = text.trim().to_string();
            if !text.is_empty() && !busy && !settings.ollama_model.is_empty() {
                self.send_chat(settings, brief, lang_id, lang_label, code, &text);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn send_chat(
        &mut self,
        settings: &Settings,
        brief: &Brief,
        lang_id: &str,
        lang_label: &str,
        code: &str,
        text: &str,
    ) {
        let mode = self.mode;
        let cx = brief.context(lang_id, lang_label);
        let system = if mode == Mode::Interview {
            self.prompts.interview_system(&cx)
        } else {
            self.prompts.guide_system(&cx)
        };

        let msgs = if mode == Mode::Interview {
            &mut self.interview
        } else {
            &mut self.guide
        };
        msgs.push(UiMsg {
            from_user: true,
            content: text.to_string(),
            thinking: String::new(),
            code_attached: self.attach,
        });
        msgs.push(UiMsg {
            from_user: false,
            ..Default::default()
        });
        self.input.clear();

        // Only the message being sent carries the code snapshot, which keeps
        // the context small enough for a local model.
        let history = msgs.clone();
        let recent: Vec<&UiMsg> = history
            .iter()
            .filter(|m| m.from_user || !m.content.is_empty())
            .rev()
            .take(HISTORY_TURNS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let mut api = vec![ChatMessage::system(system)];
        let last = recent.len().saturating_sub(1);
        for (i, m) in recent.iter().enumerate() {
            let content = if m.from_user && m.code_attached && i == last {
                attach_code(&m.content, lang_id, code)
            } else {
                m.content.clone()
            };
            api.push(if m.from_user {
                ChatMessage::user(content)
            } else {
                ChatMessage::assistant(content)
            });
        }

        self.target = Some(Target::Chat(mode));
        self.stream = Some(chat_stream(ChatOptions {
            url: settings.ollama_url.clone(),
            model: settings.ollama_model.clone(),
            messages: api,
            temperature: mode.temperature(),
        }));
    }

    #[allow(clippy::too_many_arguments)]
    fn fix_ui(
        &mut self,
        ui: &mut Ui,
        settings: &Settings,
        brief: &Brief,
        lang_id: &str,
        lang_label: &str,
        code: &str,
        run_context: &str,
    ) {
        let busy = self.busy();
        let offline = self.status == Status::Offline;

        ui.horizontal(|ui| {
            let label = if busy {
                "⏳ analyzing…"
            } else {
                "🔍 Analyze my code"
            };
            if apply_btn(
                ui,
                label,
                !offline && !busy && !settings.ollama_model.is_empty(),
            )
            .clicked()
            {
                let cx = brief.context(lang_id, lang_label);
                self.fix_snapshot = code.to_string();
                self.fix = Some(FixState::default());
                self.target = Some(Target::Fix);
                self.stream = Some(chat_stream(ChatOptions {
                    url: settings.ollama_url.clone(),
                    model: settings.ollama_model.clone(),
                    messages: vec![
                        ChatMessage::system(self.prompts.fix_system.clone()),
                        ChatMessage::user(self.prompts.fix_user(&cx, code, run_context)),
                    ],
                    temperature: Mode::Fix.temperature(),
                }));
            }
            if busy && mini_btn(ui, "■ stop").clicked() {
                self.stop();
            }
        });

        ScrollArea::vertical()
            .id_salt("ai-fix")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let Some(fix) = &self.fix else {
                    side_empty(ui, Mode::Fix.empty_hint());
                    return;
                };

                if !fix.thinking.trim().is_empty() {
                    egui::CollapsingHeader::new(
                        RichText::new("💭 thinking").size(11.0).color(TEXT_DIM),
                    )
                    .id_salt("fix-think")
                    .default_open(busy && fix.streaming.is_empty())
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(fix.thinking.trim())
                                .size(11.0)
                                .color(TEXT_DIM),
                        );
                    });
                }
                if busy && !fix.streaming.is_empty() {
                    code_block(ui, &fix.streaming, TEXT_DIM);
                }
                if !busy {
                    if !fix.analysis.is_empty() {
                        side_head(ui, "issues");
                        rich_text(ui, &fix.analysis, false, 9999);
                    }
                    if fix.proposed {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(
                                "✏ the fix is shown as a diff against your code — red is yours, \
                             green is theirs. Keep the changes you want.",
                            )
                            .size(11.5)
                            .color(GREEN),
                        );
                    }
                    if fix.no_changes {
                        side_empty(ui, "✔ the model suggested no code changes.");
                    }
                }
            });
    }
}

/// Render a reply: prose as text, fenced blocks as code — and in interview
/// mode, code collapsed behind a lock so a leaked solution is at least a
/// deliberate click.
fn rich_text(ui: &mut Ui, text: &str, hide_code: bool, salt: usize) {
    for (i, seg) in segments(text).into_iter().enumerate() {
        if !seg.code {
            if !seg.text.trim().is_empty() {
                ui.label(RichText::new(seg.text.trim()).size(12.5).color(TEXT));
            }
        } else if hide_code {
            egui::CollapsingHeader::new(
                RichText::new("🔒 code hidden — interview mode (click to peek anyway)")
                    .size(11.0)
                    .color(AMBER),
            )
            .id_salt(format!("locked-{salt}-{i}"))
            .default_open(false)
            .show(ui, |ui| code_block(ui, &seg.text, TEXT));
        } else {
            code_block(ui, &seg.text, TEXT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_land_on_the_chat_the_stream_was_started_for() {
        let mut a = Assistant {
            target: Some(Target::Chat(Mode::Guide)),
            ..Default::default()
        };
        a.guide.push(UiMsg {
            from_user: true,
            content: "q".into(),
            ..Default::default()
        });
        a.guide.push(UiMsg::default());
        a.append(Channel::Content, "hel");
        a.append(Channel::Content, "lo");
        a.append(Channel::Thinking, "hmm");
        assert_eq!(a.guide.last().unwrap().content, "hello");
        assert_eq!(a.guide.last().unwrap().thinking, "hmm");
        assert!(
            a.interview.is_empty(),
            "the other mode's transcript is untouched"
        );
    }

    #[test]
    fn switching_mode_mid_stream_does_not_misroute_tokens() {
        let mut a = Assistant {
            target: Some(Target::Chat(Mode::Interview)),
            mode: Mode::Guide, // user clicked away while it streamed
            ..Default::default()
        };
        a.interview.push(UiMsg::default());
        a.append(Channel::Content, "hint");
        assert_eq!(a.interview[0].content, "hint");
        assert!(a.guide.is_empty());
    }

    #[test]
    fn a_fix_reply_becomes_a_proposal() {
        let mut a = Assistant {
            fix_snapshot: "func f() {\n\treturn 1\n}".into(),
            fix: Some(FixState {
                streaming:
                    "ISSUES:\n- off by one\n\nFIXED CODE:\n```go\nfunc f() {\n\treturn 2\n}\n```"
                        .into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        match a.finish_fix() {
            AiAction::ProposeFix(code) => assert!(code.contains("return 2")),
            _ => panic!("expected a proposal"),
        }
        let fix = a.fix.as_ref().unwrap();
        assert!(fix.proposed);
        assert!(fix.analysis.contains("off by one"));
        assert!(!fix.analysis.contains("ISSUES:"));
    }

    #[test]
    fn an_identical_fix_is_reported_as_no_change() {
        let mut a = Assistant {
            fix_snapshot: "func f() {}".into(),
            ..Default::default()
        };
        a.fix = Some(FixState {
            streaming: "ISSUES:\n- none found\n\n```go\nfunc f() {}\n```".into(),
            ..Default::default()
        });
        assert!(matches!(a.finish_fix(), AiAction::None));
        assert!(a.fix.as_ref().unwrap().no_changes);
        assert!(!a.fix.as_ref().unwrap().proposed);
    }

    #[test]
    fn a_reply_with_no_code_block_explains_itself() {
        let mut a = Assistant {
            fix_snapshot: "x".into(),
            ..Default::default()
        };
        a.fix = Some(FixState {
            streaming: "I could not read that.".into(),
            ..Default::default()
        });
        assert!(matches!(a.finish_fix(), AiAction::None));
        assert_eq!(a.fix.as_ref().unwrap().analysis, "I could not read that.");
    }

    #[test]
    fn an_empty_model_reply_still_says_something_useful() {
        let mut a = Assistant {
            fix: Some(FixState::default()),
            ..Default::default()
        };
        a.finish_fix();
        assert!(a.fix.as_ref().unwrap().analysis.contains("nothing usable"));
    }

    #[test]
    fn stopping_clears_the_stream_and_its_target() {
        let mut a = Assistant {
            target: Some(Target::Fix),
            ..Default::default()
        };
        a.stop();
        assert!(!a.busy());
        assert!(a.target.is_none());
    }
}
