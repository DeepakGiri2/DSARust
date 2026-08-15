//! The 📘 helper — a modal drawer with two tabs: data-structure deep-dives
//! for the current category, and a cross-language syntax cheat sheet.
//!
//! Content comes from `content/guide/`, so every paragraph, complexity row and
//! code sample here is editable without touching the binary.

use crate::style::*;
use dsa_content::guide::{Guide, Topic, TopicKind};
use dsa_core::problem::LangId;
use egui::{Align, Context, Layout, RichText, ScrollArea, Ui};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Topics,
    Cheatsheet,
}

pub struct Helper {
    open: bool,
    category: String,
    tab: Tab,
    selected: String,
    /// Language shown in the syntax box; starts from the problem's language
    /// but is independent so you can compare without leaving the page.
    syntax_lang: LangId,
    cheat_from: LangId,
    cheat_to: LangId,
}

impl Default for Helper {
    fn default() -> Self {
        Self {
            open: false,
            category: String::new(),
            tab: Tab::Topics,
            selected: String::new(),
            syntax_lang: "go".into(),
            cheat_from: "go".into(),
            cheat_to: "cpp".into(),
        }
    }
}

impl Helper {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, guide: &Guide, category: &str, lang: &str) {
        self.open = true;
        self.category = category.to_string();
        self.syntax_lang = lang.to_string();
        self.cheat_from = lang.to_string();
        self.cheat_to = if lang == "cpp" {
            "go".into()
        } else {
            "cpp".into()
        };
        // Land on the first topic that actually relates to this category,
        // falling back to whatever the guide lists first.
        self.selected = guide
            .for_category(category)
            .first()
            .map(|t| t.id.clone())
            .or_else(|| guide.topics.first().map(|t| t.id.clone()))
            .unwrap_or_default();
    }

    pub fn show(&mut self, ctx: &Context, guide: &Guide, langs: &[(LangId, String)]) {
        if !self.open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
            return;
        }

        let screen = ctx.content_rect();
        let mut open = true;
        egui::Window::new("helper")
            .title_bar(false)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(screen.width() * 0.86, screen.height() * 0.86))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::default()
                    .fill(PANEL)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                self.head(ui);
                ui.separator();
                if guide.is_empty() {
                    side_empty(ui, "No guide installed — add content/guide/topics.toml.");
                    return;
                }
                match self.tab {
                    Tab::Topics => self.topics_tab(ui, guide, langs),
                    Tab::Cheatsheet => self.cheat_tab(ui, guide, langs),
                }
            });
        if !open {
            self.open = false;
        }
    }

    fn head(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("📘 helper").size(19.0).strong());
            let mut tab = self.tab;
            if seg(
                ui,
                &mut tab,
                &[
                    (Tab::Topics, "data types & techniques"),
                    (Tab::Cheatsheet, "⇄ syntax cheat sheet"),
                ],
            ) {
                self.tab = tab;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mini_btn(ui, "✖ close")
                    .on_hover_text("Close (Esc)")
                    .clicked()
                {
                    self.open = false;
                }
            });
        });
    }

    fn topics_tab(&mut self, ui: &mut Ui, guide: &Guide, langs: &[(LangId, String)]) {
        let relevant = guide.for_category(&self.category);
        let rest = guide.rest_for(&self.category);

        ui.horizontal_top(|ui| {
            // ── chips column ────────────────────────────────────────────────
            ui.vertical(|ui| {
                ui.set_width(230.0);
                ScrollArea::vertical()
                    .id_salt("chips")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("for “{}”", self.category))
                                .size(11.0)
                                .color(TEXT_DIM),
                        );
                        ui.add_space(4.0);
                        for t in &relevant {
                            self.chip(ui, t);
                        }
                        if !rest.is_empty() {
                            ui.add_space(10.0);
                            ui.label(RichText::new("everything else").size(11.0).color(TEXT_DIM));
                            ui.add_space(4.0);
                            for t in &rest {
                                self.chip(ui, t);
                            }
                        }
                    });
            });

            ui.separator();

            // ── topic body ──────────────────────────────────────────────────
            let selected = guide
                .topic(&self.selected)
                .or_else(|| relevant.first().copied())
                .or_else(|| guide.topics.first());
            let Some(topic) = selected else { return };

            ui.vertical(|ui| {
                ScrollArea::vertical()
                    .id_salt("topic")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("{} {}", topic.emoji, topic.title))
                                .size(20.0)
                                .strong(),
                        );
                        ui.add_space(10.0);

                        for para in &topic.what {
                            ui.label(RichText::new(para).size(13.0).color(TEXT));
                            ui.add_space(8.0);
                        }

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("⏱ time complexity")
                                .size(12.5)
                                .strong()
                                .color(ACCENT2),
                        );
                        ui.add_space(4.0);
                        complexity_table(ui, topic);

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✏ syntax").size(12.5).strong().color(ACCENT2));
                            let opts: Vec<(LangId, &str)> = langs
                                .iter()
                                .map(|(id, label)| (id.clone(), label.as_str()))
                                .collect();
                            let mut cur = self.syntax_lang.clone();
                            if seg(ui, &mut cur, &opts) {
                                self.syntax_lang = cur;
                            }
                        });
                        ui.add_space(4.0);
                        let code = topic
                            .syntax
                            .get(&self.syntax_lang)
                            .cloned()
                            .unwrap_or_else(|| "(no sample for this language)".into());
                        code_block(ui, &code, TEXT);

                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("⚠ indexing, ranges & pitfalls")
                                .size(12.5)
                                .strong()
                                .color(AMBER),
                        );
                        ui.add_space(4.0);
                        for note in &topic.notes {
                            ui.horizontal_top(|ui| {
                                ui.label(RichText::new("•").size(13.0).color(TEXT_DIM));
                                ui.label(RichText::new(note).size(12.5).color(TEXT));
                            });
                            ui.add_space(3.0);
                        }
                    });
            });
        });
    }

    fn chip(&mut self, ui: &mut Ui, topic: &Topic) {
        let on = self.selected == topic.id;
        let label = format!("{} {}", topic.emoji, topic.title);
        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(12.0).color(if on {
                egui::Color32::WHITE
            } else {
                TEXT
            }))
            .fill(if on { ACCENT } else { PANEL2 })
            .stroke(egui::Stroke::new(1.0, if on { ACCENT } else { BORDER }))
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(egui::vec2(ui.available_width(), 0.0)),
        );
        if topic.kind == TopicKind::Technique {
            resp.clone().on_hover_text("technique");
        }
        if resp.clicked() {
            self.selected = topic.id.clone();
        }
        ui.add_space(3.0);
    }

    fn cheat_tab(&mut self, ui: &mut Ui, guide: &Guide, langs: &[(LangId, String)]) {
        let opts: Vec<(LangId, &str)> = langs
            .iter()
            .map(|(id, label)| (id.clone(), label.as_str()))
            .collect();

        ui.horizontal(|ui| {
            ui.label(RichText::new("I know").size(12.5).color(TEXT_DIM));
            let mut from = self.cheat_from.clone();
            if seg(ui, &mut from, &opts) {
                self.cheat_from = from;
            }
            ui.label(RichText::new("→ show me").size(12.5).color(TEXT_DIM));
            let mut to = self.cheat_to.clone();
            if seg(ui, &mut to, &opts) {
                self.cheat_to = to;
            }
            if mini_btn(ui, "⇄ swap")
                .on_hover_text("Swap the two languages")
                .clicked()
            {
                std::mem::swap(&mut self.cheat_from, &mut self.cheat_to);
            }
        });
        ui.add_space(8.0);

        let label_of = |id: &str| {
            langs
                .iter()
                .find(|(l, _)| l == id)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| id.into())
        };

        ScrollArea::vertical()
            .id_salt("cheat")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for section in &guide.cheatsheet {
                    ui.add_space(10.0);
                    ui.label(RichText::new(&section.name).size(15.0).strong());
                    ui.add_space(4.0);
                    egui::Grid::new(format!("cheat-{}", section.name))
                        .num_columns(3)
                        .spacing([12.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("").size(11.0));
                            ui.label(
                                RichText::new(label_of(&self.cheat_from))
                                    .size(11.5)
                                    .strong()
                                    .color(ACCENT),
                            );
                            ui.label(
                                RichText::new(label_of(&self.cheat_to))
                                    .size(11.5)
                                    .strong()
                                    .color(ACCENT2),
                            );
                            ui.end_row();

                            for row in &section.rows {
                                ui.label(RichText::new(&row.topic).size(12.0).color(TEXT_DIM));
                                let get = |id: &str| {
                                    row.code.get(id).cloned().unwrap_or_else(|| "—".into())
                                };
                                ui.label(
                                    RichText::new(get(&self.cheat_from)).monospace().size(11.5),
                                );
                                ui.label(RichText::new(get(&self.cheat_to)).monospace().size(11.5));
                                ui.end_row();
                            }
                        });
                }
            });
    }
}

fn complexity_table(ui: &mut Ui, topic: &Topic) {
    egui::Grid::new(format!("cx-{}", topic.id))
        .num_columns(3)
        .spacing([14.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new("operation").size(11.0).color(TEXT_DIM));
            ui.label(RichText::new("time").size(11.0).color(TEXT_DIM));
            ui.label(RichText::new("").size(11.0));
            ui.end_row();
            for row in &topic.complexity {
                ui.label(RichText::new(&row.op).size(12.0));
                ui.label(
                    RichText::new(&row.big)
                        .monospace()
                        .size(12.5)
                        .strong()
                        .color(ACCENT2),
                );
                ui.label(RichText::new(&row.note).size(11.5).color(TEXT_DIM));
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_defaults_the_cheat_sheet_to_a_different_language() {
        let mut h = Helper::default();
        let guide = Guide::default();
        h.open(&guide, "Trees", "go");
        assert_eq!(h.cheat_from, "go");
        assert_ne!(
            h.cheat_to, h.cheat_from,
            "comparing a language with itself is useless"
        );
        h.open(&guide, "Trees", "cpp");
        assert_eq!(h.cheat_to, "go");
    }

    #[test]
    fn opening_carries_the_problems_language_into_the_syntax_box() {
        let mut h = Helper::default();
        h.open(&Guide::default(), "Trees", "java");
        assert_eq!(h.syntax_lang, "java");
        assert!(h.is_open());
    }
}
