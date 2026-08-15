//! The ⏵ Visualize mode: code + animation + inspector, with the transport
//! controls along the bottom.
//!
//! Layout follows the web version's `.pv-grid`: code and input on the left,
//! the note bar and canvas in the middle, variables / call stack / logs on the
//! right. Opening a problem lands on Practice first, so the walkthrough stays
//! behind an explicit reveal and clicking a problem never spoils it.

use crate::highlight::{line_job, Palette};
use crate::settings::Settings;
use crate::style::*;
use dsa_content::Library;
use dsa_core::model::{LogEntry, LogKind, StepEvent, Trace, VarVal};
use dsa_core::problem::{validate_inputs, InputMap};
use dsa_core::{StepMode, Timeline};
use dsa_viz::Theme;
use egui::{Align, Key, Layout, RichText, ScrollArea, Ui};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Visualize {
    pub trace: Trace,
    pub timeline: Timeline,
    pub trace_error: Option<String>,
    pub inputs: BTreeMap<String, String>,
    pub input_errors: Vec<String>,
    frame_sel: Option<usize>,
    console_open: bool,
    /// The walkthrough is gated until the user asks for it.
    pub revealed: bool,
    last_idx: usize,
}

impl Visualize {
    /// Point at a problem: load its default input and record the first trace.
    pub fn load(&mut self, lib: &Library, slug: &str, settings: &Settings) {
        self.frame_sel = None;
        self.console_open = false;
        self.revealed = false;
        self.inputs.clear();
        if let Some(pack) = lib.pack(slug) {
            for f in &pack.meta.inputs {
                let text = pack
                    .meta
                    .default_input
                    .get(&f.name)
                    .map(|v| v.to_editable())
                    .unwrap_or_default();
                self.inputs.insert(f.name.clone(), text);
            }
        }
        self.retrace(lib, slug, settings, true);
    }

    pub fn retrace(&mut self, lib: &Library, slug: &str, settings: &Settings, reset: bool) {
        self.trace_error = None;
        self.input_errors.clear();

        let Some(pack) = lib.pack(slug) else {
            self.trace = Trace::default();
            self.timeline.reset(0);
            return;
        };
        if !pack.has_script() {
            self.trace = Trace::default();
            self.timeline.reset(0);
            self.trace_error =
                Some("This problem ships code and tests, but no animation yet.".into());
            return;
        }

        let mut values = InputMap::new();
        for f in &pack.meta.inputs {
            let raw = self.inputs.get(&f.name).cloned().unwrap_or_default();
            match f.parse(&raw) {
                Ok(v) => {
                    values.insert(f.name.clone(), v);
                }
                Err(e) => self.input_errors.push(e),
            }
        }
        self.input_errors
            .extend(validate_inputs(&pack.meta.inputs, &values));
        if let Some(e) = lib.validate(slug, &values) {
            self.input_errors.push(e);
        }
        if !self.input_errors.is_empty() {
            return;
        }

        match lib.trace(slug, &values) {
            Ok(trace) => {
                let len = trace.len();
                self.trace = trace;
                if reset {
                    self.timeline.reset(len);
                } else {
                    self.timeline.retarget(len);
                }
                self.timeline.speed = settings.speed;
                self.timeline.animate = settings.animate;
            }
            Err(e) => {
                self.trace = Trace::default();
                self.timeline.reset(0);
                self.trace_error = Some(e.to_string());
            }
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let moved = self.timeline.idx() != self.last_idx;
        if moved {
            self.frame_sel = None;
            self.last_idx = self.timeline.idx();
        }
        self.timeline.tick(dt) || self.timeline.needs_repaint() || moved
    }

    pub fn keys(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() || !self.revealed {
            return;
        }
        let trace = std::mem::take(&mut self.trace);
        ctx.input(|i| {
            if i.key_pressed(Key::Space) {
                self.timeline.toggle_play();
            }
            if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::F10) {
                self.timeline.step(StepMode::Over, &trace);
            }
            if i.key_pressed(Key::ArrowDown) || (i.key_pressed(Key::F11) && !i.modifiers.shift) {
                self.timeline.step(StepMode::In, &trace);
            }
            if i.key_pressed(Key::ArrowUp) || (i.key_pressed(Key::F11) && i.modifiers.shift) {
                self.timeline.step(StepMode::Out, &trace);
            }
            if i.key_pressed(Key::ArrowLeft) {
                self.timeline.step(StepMode::Back, &trace);
            }
            if i.key_pressed(Key::C) {
                self.timeline.continue_run();
            }
            if i.key_pressed(Key::R) {
                self.timeline.restart();
            }
        });
        self.trace = trace;
    }

    // ── the reveal gate ─────────────────────────────────────────────────────

    /// Returns true when the user chose to go back to Practice.
    pub fn gate(&mut self, ui: &mut Ui, lang_label: &str) -> bool {
        let mut to_practice = false;
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(560.0);
            card(ui, |ui| {
                ui.label(RichText::new("🎓 Practice first").size(20.0).strong());
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "This tab plays the full solution step by step — reading it before trying robs you \
                         of the struggle that makes it stick. Attempt it yourself in Practice: write real \
                         {lang_label}, run it against the tests, get stuck, think."
                    ))
                    .size(13.0)
                    .color(TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Stuck on the underlying data structure? The 📘 helper explains it (with syntax \
                         and complexity) without spoiling this problem.",
                    )
                    .size(12.0)
                    .color(TEXT_DIM),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if apply_btn(ui, "✏ Let me solve it first", true).clicked() {
                        to_practice = true;
                    }
                    if mini_btn(ui, "👀 I tried / I'm stuck — show the walkthrough").clicked() {
                        self.revealed = true;
                    }
                });
            });
        });
        to_practice
    }

    // ── the grid ────────────────────────────────────────────────────────────

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        lib: &Library,
        settings: &mut Settings,
        theme: &Theme,
        slug: &str,
        lang: &str,
    ) {
        egui::TopBottomPanel::bottom("controls").show_inside(ui, |ui| {
            self.controls(ui, settings);
        });

        egui::SidePanel::left("code-col")
            .resizable(true)
            .default_width(430.0)
            .width_range(300.0..=760.0)
            .show_inside(ui, |ui| {
                let label = lib
                    .language(lang)
                    .map(|l| l.label.clone())
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("code — {label}"))
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("click ○ to set a breakpoint")
                                .size(10.0)
                                .color(TEXT_DIM),
                        );
                    });
                });
                self.code_panel(ui, lib, slug, lang, theme);
                ui.separator();
                self.input_panel(ui, lib, slug, settings);
            });

        egui::SidePanel::right("inspect-col")
            .resizable(true)
            .default_width(320.0)
            .width_range(240.0..=520.0)
            .show_inside(ui, |ui| self.inspector(ui, settings, slug));

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.note_bar(ui);
            if let Some(err) = self.trace_error.clone() {
                card(ui, |ui| {
                    ui.label(RichText::new(err).monospace().size(12.0).color(RED));
                });
                return;
            }
            let idx = self.timeline.idx();
            let from = self.timeline.from_idx();
            let t = self.timeline.transition();
            let cur = self
                .trace
                .get(idx)
                .map(|s| s.views.clone())
                .unwrap_or_default();
            let prev = if from != idx {
                self.trace.get(from).map(|s| s.views.clone())
            } else {
                None
            };

            ScrollArea::vertical()
                .id_salt("canvas")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(6.0);
                    dsa_viz::show_views(ui, &cur, prev.as_deref(), t, theme);
                    if self.timeline.at_end() {
                        if let Some(result) = self.trace.result.clone() {
                            ui.add_space(6.0);
                            card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("result").size(11.0).color(TEXT_DIM));
                                    ui.label(
                                        RichText::new(result).monospace().size(14.0).color(GREEN),
                                    );
                                });
                            });
                        }
                    }
                    if let Some(pack) = lib.pack(slug) {
                        ui.add_space(10.0);
                        card(ui, |ui| {
                            ui.label(RichText::new("Problem.").size(12.0).strong().color(ACCENT));
                            ui.label(RichText::new(&pack.meta.description).size(12.5).color(TEXT));
                            ui.add_space(6.0);
                            ui.label(RichText::new("Approach.").size(12.0).strong().color(ACCENT));
                            ui.label(RichText::new(&pack.meta.approach).size(12.5).color(TEXT));
                        });
                    }
                });
        });
    }

    fn note_bar(&mut self, ui: &mut Ui) {
        let step = self.trace.get(self.timeline.idx()).cloned();
        let breakpoint = self.timeline.breakpoints.contains(&self.timeline.idx());
        egui::Frame::default()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let (glyph, color) = match step.as_ref().map(|s| s.event) {
                        Some(StepEvent::Call) => ("⤵", ACCENT2),
                        Some(StepEvent::Return) => ("⤴", GREEN),
                        _ => ("·", TEXT_DIM),
                    };
                    ui.label(RichText::new(glyph).size(13.0).color(color));
                    ui.label(
                        RichText::new(step.as_ref().map(|s| s.note.clone()).unwrap_or_default())
                            .size(13.0)
                            .color(TEXT),
                    );
                    if breakpoint {
                        pill(ui, "● breakpoint", RED, tint(RED));
                    }
                });
            });
        ui.add_space(6.0);
    }

    fn code_panel(&mut self, ui: &mut Ui, lib: &Library, slug: &str, lang: &str, theme: &Theme) {
        let Some(parsed) = lib.pack(slug).and_then(|p| p.source(lang)) else {
            side_empty(ui, "No source for this language.");
            return;
        };
        let syntax = lib
            .language(lang)
            .map(|l| l.syntax.clone())
            .unwrap_or_else(|| lang.into());
        let lines: Vec<String> = parsed.clean.lines().map(|s| s.to_string()).collect();
        let tags: Vec<Option<String>> = (1..=lines.len())
            .map(|n| parsed.tag_at(n).map(|s| s.to_string()))
            .collect();
        let cur_line = self
            .trace
            .get(self.timeline.idx())
            .and_then(|s| parsed.line_of(&s.tag));

        let palette = Palette {
            text: theme.text,
            keyword: theme.window,
            type_name: theme.accent,
            string: theme.good,
            number: theme.cur,
            comment: theme.dim,
            punct: theme.muted,
        };
        let trace = std::mem::take(&mut self.trace);

        ScrollArea::both()
            .id_salt("code")
            .max_height(ui.available_height() * 0.62)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                for (i, line) in lines.iter().enumerate() {
                    let n = i + 1;
                    let is_cur = cur_line == Some(n);
                    let tag = tags[i].clone();
                    let has_bp = tag
                        .as_ref()
                        .is_some_and(|t| self.timeline.has_breakpoint_tag(t, &trace));

                    let row = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let dot = match (&tag, has_bp) {
                            (Some(_), true) => RichText::new("●").size(11.0).color(RED),
                            (Some(_), false) => RichText::new("○").size(11.0).color(TEXT_DIM),
                            (None, _) => RichText::new(" ").size(11.0),
                        };
                        let bp = ui.add(egui::Button::new(dot).frame(false));
                        if bp.clicked() {
                            if let Some(t) = &tag {
                                self.timeline.toggle_breakpoint_tag(t, &trace);
                            }
                        }
                        ui.label(
                            RichText::new(format!("{n:>3}"))
                                .monospace()
                                .size(11.0)
                                .color(TEXT_DIM),
                        );
                        ui.label(line_job(line, &syntax, 12.5, &palette));
                    });

                    if is_cur {
                        let r = row.response.rect;
                        ui.painter().rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(ui.min_rect().left(), r.top() - 1.0),
                                egui::pos2(ui.min_rect().right(), r.bottom() + 1.0),
                            ),
                            4.0,
                            tint(ACCENT),
                        );
                        row.response.scroll_to_me(Some(Align::Center));
                    }
                }
            });
        self.trace = trace;
    }

    fn input_panel(&mut self, ui: &mut Ui, lib: &Library, slug: &str, settings: &Settings) {
        let Some(pack) = lib.pack(slug) else { return };
        let fields: Vec<(String, String, Option<String>)> = pack
            .meta
            .inputs
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    f.display_label().to_string(),
                    f.help.clone(),
                )
            })
            .collect();
        if fields.is_empty() {
            return;
        }

        side_head(ui, "input");
        let mut apply = false;
        ui.horizontal_wrapped(|ui| {
            for (name, label, help) in &fields {
                ui.label(RichText::new(label).size(11.5).color(TEXT_DIM));
                let text = self.inputs.entry(name.clone()).or_default();
                let width = if text.len() > 20 { 200.0 } else { 110.0 };
                let resp = ui.add(
                    egui::TextEdit::singleline(text)
                        .desired_width(width)
                        .font(egui::TextStyle::Monospace),
                );
                if let Some(h) = help {
                    resp.clone().on_hover_text(h);
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    apply = true;
                }
            }
            if apply_btn(ui, "re-visualize", true).clicked() {
                apply = true;
            }
        });
        for e in self.input_errors.clone() {
            ui.label(RichText::new(e).size(11.5).color(RED));
        }
        if apply {
            self.retrace(lib, slug, settings, true);
        }
    }

    fn inspector(&mut self, ui: &mut Ui, settings: &mut Settings, slug: &str) {
        let Some(step) = self.trace.get(self.timeline.idx()).cloned() else {
            side_empty(ui, "No trace.");
            return;
        };
        let frames = step.frames.len();
        let idx = self.frame_sel.unwrap_or(frames.saturating_sub(1));

        ui.horizontal(|ui| {
            ui.label(RichText::new("variables").size(11.0).color(TEXT_DIM));
            if idx + 1 != frames {
                if let Some(f) = step.frames.get(idx) {
                    ui.label(
                        RichText::new(format!("(frame: {})", f.fn_name))
                            .size(10.5)
                            .color(AMBER),
                    );
                }
            }
        });

        let prev_vars = self
            .timeline
            .idx()
            .checked_sub(1)
            .and_then(|i| self.trace.get(i))
            .and_then(|s| s.frames.get(idx))
            .map(|f| f.vars.clone());

        ScrollArea::vertical()
            .id_salt("vars")
            .max_height(240.0)
            .auto_shrink([false, false])
            .show(ui, |ui| match step.frames.get(idx) {
                Some(frame) if !frame.vars.is_empty() => {
                    for (name, value) in frame.vars.iter() {
                        let changed = prev_vars
                            .as_ref()
                            .and_then(|v| v.get(name))
                            .map(|old| old != value)
                            .unwrap_or(true);
                        var_row(ui, settings, slug, name, value, changed);
                    }
                }
                _ => side_empty(ui, "no variables in this frame"),
            });

        side_head(ui, &format!("call stack · depth {}", step.depth));
        for (i, frame) in step.frames.iter().enumerate().rev() {
            let selected = idx == i;
            let text = RichText::new(format!("{}{}", "  ".repeat(i), frame.fn_name))
                .monospace()
                .size(11.5)
                .color(if i + 1 == frames { TEXT } else { TEXT_DIM });
            if ui.selectable_label(selected, text).clicked() {
                self.frame_sel = Some(i);
            }
        }

        side_head(ui, "logs");
        ScrollArea::vertical()
            .id_salt("logs")
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in self.trace.logs_at(self.timeline.idx()) {
                    let color = match entry.kind {
                        dsa_core::model::LogKind::Result => GREEN,
                        dsa_core::model::LogKind::Call => ACCENT2,
                        dsa_core::model::LogKind::Return => ACCENT,
                        _ => TEXT_DIM,
                    };
                    ui.label(
                        RichText::new(&entry.text)
                            .monospace()
                            .size(10.5)
                            .color(color),
                    );
                }
            });
    }

    fn controls(&mut self, ui: &mut Ui, settings: &mut Settings) {
        ui.add_space(4.0);
        let trace = std::mem::take(&mut self.trace);
        toolbar(ui, |ui| {
            let on = !trace.is_empty();
            ui.add_enabled_ui(on, |ui| {
                if mini_btn(ui, "⏮").on_hover_text("restart (R)").clicked() {
                    self.timeline.restart();
                }
                if mini_btn(ui, "◀").on_hover_text("step back (←)").clicked() {
                    self.timeline.step(StepMode::Back, &trace);
                }
                let playing = self.timeline.playing();
                if apply_btn(ui, if playing { "⏸" } else { "▶" }, true)
                    .on_hover_text("play / pause (space)")
                    .clicked()
                {
                    self.timeline.toggle_play();
                }
                if mini_btn(ui, "⤵ in")
                    .on_hover_text("step in (↓ / F11)")
                    .clicked()
                {
                    self.timeline.step(StepMode::In, &trace);
                }
                if mini_btn(ui, "↪ over")
                    .on_hover_text("step over (→ / F10)")
                    .clicked()
                {
                    self.timeline.step(StepMode::Over, &trace);
                }
                if mini_btn(ui, "⤴ out")
                    .on_hover_text("step out (↑ / shift+F11)")
                    .clicked()
                {
                    self.timeline.step(StepMode::Out, &trace);
                }
                if mini_btn(ui, "▶▶")
                    .on_hover_text("continue to the next breakpoint (C)")
                    .clicked()
                {
                    self.timeline.continue_run();
                }
                if mini_btn(ui, "⏭").on_hover_text("jump to the end").clicked() {
                    self.timeline.to_end();
                }

                let mut idx = self.timeline.idx();
                let max = trace.len().saturating_sub(1).max(1);
                if ui
                    .add(
                        egui::Slider::new(&mut idx, 0..=max)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    self.timeline.jump_to(idx);
                }
                ui.label(
                    RichText::new(format!("{} / {}", self.timeline.idx() + 1, trace.len()))
                        .monospace()
                        .size(11.0)
                        .color(TEXT_DIM),
                );

                ui.label(RichText::new("speed").size(11.0).color(TEXT_DIM));
                if ui
                    .add(
                        egui::Slider::new(&mut settings.speed, 0.25..=6.0)
                            .logarithmic(true)
                            .show_value(false),
                    )
                    .changed()
                {
                    self.timeline.speed = settings.speed;
                }
                ui.label(
                    RichText::new(format!("{:.2}x", settings.speed))
                        .size(10.5)
                        .color(TEXT_DIM),
                );
                if ui
                    .checkbox(&mut settings.animate, RichText::new("animate").size(11.5))
                    .changed()
                {
                    self.timeline.animate = settings.animate;
                }
                if toggle_btn(ui, "⌨ console", self.console_open).clicked() {
                    self.console_open = !self.console_open;
                }
            });
        });
        if self.console_open {
            console_panel(ui, &trace, self.timeline.idx());
        }
        self.trace = trace;
        ui.add_space(4.0);
    }
}

/// The ⌨ console, built from the same vocabulary as the Practice tab's OUTPUT
/// panel: a small header with a right-aligned status, a framed monospace body,
/// and a real empty state rather than a blank rectangle.
fn console_panel(ui: &mut Ui, trace: &Trace, step: usize) {
    let logs = trace.logs_at(step);
    let fresh = logs.iter().filter(|e| e.step == step).count();

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("CONSOLE").size(11.0).color(TEXT_DIM));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if fresh > 0 {
                pill(ui, &format!("+{fresh} this step"), ACCENT2, tint(ACCENT2));
            }
            ui.label(
                RichText::new(format!(
                    "{} line{}",
                    logs.len(),
                    if logs.len() == 1 { "" } else { "s" }
                ))
                .size(10.5)
                .color(TEXT_DIM),
            );
        });
    });
    ui.add_space(2.0);

    egui::Frame::default()
        .fill(PANEL2)
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            if logs.is_empty() {
                side_empty(
                    ui,
                    "nothing logged yet — lines appear as the animation runs",
                );
                return;
            }
            // Hug the content vertically: the frame is visible now, so always
            // claiming the full height would leave a large empty box early in a
            // trace. Width still fills so the rows line up.
            ScrollArea::vertical()
                .id_salt("console")
                .max_height(120.0)
                .stick_to_bottom(true)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for entry in logs {
                        console_row(ui, entry, entry.step == step);
                    }
                });
        });
}

/// One console line. Lines emitted during the current step keep their full
/// colour and everything earlier fades, so stepping forward shows at a glance
/// what *this* step did rather than an undifferentiated wall of history.
fn console_row(ui: &mut Ui, entry: &LogEntry, current: bool) {
    let accent = log_color(entry.kind);
    let (glyph_c, text_c) = if current {
        let text = if entry.kind == LogKind::Log {
            TEXT
        } else {
            accent
        };
        (accent, text)
    } else {
        (accent.gamma_multiply(0.5), TEXT_DIM.gamma_multiply(0.85))
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(
            RichText::new(format!("{:>3}", entry.step + 1))
                .monospace()
                .size(10.0)
                .color(TEXT_DIM.gamma_multiply(0.6)),
        );
        ui.label(
            RichText::new(log_glyph(entry.kind))
                .size(11.0)
                .color(glyph_c),
        );
        ui.label(
            RichText::new(log_text(entry))
                .monospace()
                .size(11.0)
                .color(text_c),
        );
    });
}

fn var_row(
    ui: &mut Ui,
    settings: &mut Settings,
    slug: &str,
    name: &str,
    value: &VarVal,
    changed: bool,
) {
    let watched = settings.is_watched(slug, name);
    ui.horizontal(|ui| {
        let star = if watched { "★" } else { "☆" };
        if ui
            .add(
                egui::Button::new(RichText::new(star).size(11.0).color(if watched {
                    AMBER
                } else {
                    TEXT_DIM
                }))
                .frame(false),
            )
            .on_hover_text("pin to watch")
            .clicked()
        {
            settings.toggle_watch(slug, name);
        }
        ui.label(RichText::new(name).monospace().size(11.5).color(ACCENT2));
        ui.label(
            RichText::new(value.summary())
                .monospace()
                .size(11.5)
                .color(if changed { AMBER } else { TEXT }),
        );
    });
    if value.is_composite() {
        let lines: Vec<String> = match value {
            VarVal::Map { v, hl } => v
                .iter()
                .map(|(k, val)| format!("{} {k} → {val}", if hl.contains(k) { "▸" } else { " " }))
                .collect(),
            VarVal::Set { v, .. } => v.iter().map(|x| format!("  {x}")).collect(),
            _ => vec![],
        };
        for line in lines.iter().take(14) {
            ui.label(
                RichText::new(format!("    {line}"))
                    .monospace()
                    .size(10.5)
                    .color(TEXT_DIM),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_freshly_opened_problem_is_gated() {
        let v = Visualize::default();
        assert!(
            !v.revealed,
            "clicking a problem must not spoil the solution"
        );
    }

    #[test]
    fn moving_the_cursor_clears_a_pinned_frame_selection() {
        let mut v = Visualize {
            frame_sel: Some(0),
            timeline: Timeline::new(5),
            ..Default::default()
        };
        v.timeline.jump_to(2);
        v.tick(0.016);
        assert!(
            v.frame_sel.is_none(),
            "a new step invalidates the selected frame"
        );
    }
}
