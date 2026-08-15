//! "Who's practising?" — the profile picker.
//!
//! Netflix's model, and for the same reason: one machine, several people, and
//! nothing worth putting behind a password. A profile is a name, a face and a
//! colour; what it *holds* is a private copy of the catalogue's progress, its
//! own favourites and its own playlists. There is no account, no sign-in and no
//! limit on how many exist.
//!
//! The screen has two modes. Normally the cards are doors — click one and you
//! are in. Under "manage" they become editable, which is where renaming,
//! re-facing and deleting live, so a mis-click on the way in can never delete
//! six months of progress.

use crate::progress::Progress;
use crate::style::*;
use dsa_store::{Profile, AVATARS, COLORS};
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, ScrollArea, Sense, Stroke, Ui};

const CARD: f32 = 132.0;

pub enum ProfileAction {
    None,
    /// Start using this profile.
    Enter(i64),
}

/// The profile being written in the editor sheet.
struct Draft {
    /// `None` while creating.
    id: Option<i64>,
    name: String,
    avatar: String,
    color: String,
    confirm_delete: bool,
}

impl Draft {
    fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            avatar: AVATARS[0].into(),
            color: COLORS[0].into(),
            confirm_delete: false,
        }
    }
    fn of(p: &Profile) -> Self {
        Self {
            id: Some(p.id),
            name: p.name.clone(),
            avatar: p.avatar.clone(),
            color: p.color.clone(),
            confirm_delete: false,
        }
    }
}

#[derive(Default)]
pub struct Screen {
    draft: Option<Draft>,
    manage: bool,
    /// Why the last save did not take, shown under the name field.
    error: Option<String>,
}

impl Screen {
    /// Open straight into "create your first profile", for a first run.
    pub fn start_creating(&mut self) {
        self.draft = Some(Draft::new());
        self.error = None;
    }

    pub fn show(&mut self, ui: &mut Ui, progress: &mut Progress) -> ProfileAction {
        let mut action = ProfileAction::None;
        let profiles = progress.profiles();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(64.0);
                // The editor is a form and wants to be narrow; the picker is a
                // wall of cards and wants the room. Both are centred.
                let want = if self.draft.is_some() { 560.0 } else { 980.0 };
                let width = ui.available_width().min(want);
                let pad = ((ui.available_width() - width) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_max_width(width);
                        if self.draft.is_some() {
                            self.editor(ui, progress, &profiles);
                        } else {
                            action = self.picker(ui, progress, &profiles);
                        }
                    });
                });
                ui.add_space(48.0);
            });

        action
    }

    // ── picking ─────────────────────────────────────────────────────────────

    fn picker(
        &mut self,
        ui: &mut Ui,
        progress: &mut Progress,
        profiles: &[Profile],
    ) -> ProfileAction {
        let mut action = ProfileAction::None;

        ui.vertical_centered(|ui| {
            gradient_heading(
                ui,
                if profiles.is_empty() {
                    "Make a "
                } else {
                    "Who's "
                },
                if profiles.is_empty() {
                    "profile"
                } else {
                    "practising?"
                },
                40.0,
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Each profile keeps its own progress, favourites and playlists. \
                     No account, no password — just a name.",
                )
                .size(13.0)
                .color(TEXT_DIM),
            );
        });
        ui.add_space(32.0);

        // Cards wrap, so twenty profiles are as usable as two. The row is
        // centred by hand: `horizontal_wrapped` left-aligns, and three cards
        // hugging the left edge of a wide window look like a mistake.
        const GAP: f32 = 18.0;
        let per_row = ((ui.available_width() + GAP) / (CARD + GAP))
            .floor()
            .max(1.0);
        let in_row = ((profiles.len() + 1) as f32).min(per_row);
        let row_width = in_row * CARD + (in_row - 1.0) * GAP;
        let indent = ((ui.available_width() - row_width) * 0.5).max(0.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
            ui.add_space(indent);
            for profile in profiles {
                let stats = progress.stats_for(profile.id);
                let subtitle = if stats.solved == 0 {
                    "not started".to_string()
                } else {
                    format!("{} solved", stats.solved)
                };
                if self.card(ui, &profile.avatar, &profile.name, &subtitle, profile) {
                    if self.manage {
                        self.draft = Some(Draft::of(profile));
                        self.error = None;
                    } else {
                        action = ProfileAction::Enter(profile.id);
                    }
                }
            }
            if self.add_card(ui) {
                self.draft = Some(Draft::new());
                self.error = None;
            }
        });

        if !profiles.is_empty() {
            ui.add_space(34.0);
            ui.vertical_centered(|ui| {
                if toggle_btn(
                    ui,
                    if self.manage {
                        "✔ done"
                    } else {
                        "⚙ manage profiles"
                    },
                    self.manage,
                )
                .on_hover_text("Rename, re-face or delete a profile")
                .clicked()
                {
                    self.manage = !self.manage;
                }
            });
        }

        action
    }

    /// One profile tile: a coloured face, the name under it, and what they have
    /// done so far. Returns true when it is clicked.
    fn card(
        &self,
        ui: &mut Ui,
        avatar: &str,
        name: &str,
        subtitle: &str,
        profile: &Profile,
    ) -> bool {
        let accent = parse_color(&profile.color);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(CARD, CARD + 46.0), Sense::click());
        let hovered = response.hovered();
        let face = Rect::from_min_size(rect.min, egui::vec2(CARD, CARD));
        let p = ui.painter();

        // Netflix grows the tile it is pointing at; the border doing the work
        // here keeps the layout still, which matters when the cards wrap.
        p.rect_filled(face, CornerRadius::same(14), alpha(accent, 0x2e));
        p.rect_stroke(
            face,
            CornerRadius::same(14),
            Stroke::new(
                if hovered { 2.0 } else { 1.0 },
                alpha(accent, if hovered { 0xff } else { 0x66 }),
            ),
            egui::StrokeKind::Inside,
        );
        p.text(
            face.center(),
            egui::Align2::CENTER_CENTER,
            avatar,
            egui::FontId::proportional(52.0),
            accent,
        );

        if self.manage {
            // A pencil in the corner, so "manage" is visible on the card and
            // not only in the button that turned it on.
            p.text(
                face.right_top() + egui::vec2(-14.0, 14.0),
                egui::Align2::CENTER_CENTER,
                "✏",
                egui::FontId::proportional(15.0),
                TEXT,
            );
        }

        p.text(
            egui::pos2(face.center().x, face.bottom() + 15.0),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(14.5),
            if hovered { TEXT } else { alpha(TEXT, 0xdd) },
        );
        p.text(
            egui::pos2(face.center().x, face.bottom() + 34.0),
            egui::Align2::CENTER_CENTER,
            subtitle,
            egui::FontId::proportional(11.0),
            TEXT_DIM,
        );

        response.clicked()
    }

    fn add_card(&self, ui: &mut Ui) -> bool {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(CARD, CARD + 46.0), Sense::click());
        let face = Rect::from_min_size(rect.min, egui::vec2(CARD, CARD));
        let hovered = response.hovered();
        let p = ui.painter();
        p.rect_filled(face, CornerRadius::same(14), alpha(PANEL2, 0xcc));
        p.rect_stroke(
            face,
            CornerRadius::same(14),
            Stroke::new(1.0, if hovered { ACCENT } else { BORDER }),
            egui::StrokeKind::Inside,
        );
        p.text(
            face.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(46.0),
            if hovered { ACCENT } else { TEXT_DIM },
        );
        p.text(
            egui::pos2(face.center().x, face.bottom() + 15.0),
            egui::Align2::CENTER_CENTER,
            "add profile",
            egui::FontId::proportional(14.5),
            TEXT_DIM,
        );
        response.clicked()
    }

    // ── creating and editing ────────────────────────────────────────────────

    fn editor(&mut self, ui: &mut Ui, progress: &mut Progress, profiles: &[Profile]) {
        let Some(draft) = &mut self.draft else { return };
        let editing = draft.id.is_some();

        ui.vertical_centered(|ui| {
            gradient_heading(ui, if editing { "Edit " } else { "New " }, "profile", 34.0);
        });
        ui.add_space(22.0);

        let mut close = false;
        let mut save = false;
        let mut delete = false;

        card(ui, |ui| {
            ui.horizontal(|ui| {
                let accent = parse_color(&draft.color);
                let (face, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(face, CornerRadius::same(14), alpha(accent, 0x2e));
                p.rect_stroke(
                    face,
                    CornerRadius::same(14),
                    Stroke::new(1.0, alpha(accent, 0xaa)),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    face.center(),
                    egui::Align2::CENTER_CENTER,
                    &draft.avatar,
                    egui::FontId::proportional(40.0),
                    accent,
                );

                ui.add_space(14.0);
                ui.vertical(|ui| {
                    side_head(ui, "NAME");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut draft.name)
                            .hint_text("who is this?")
                            .desired_width(280.0),
                    );
                    // Enter saves, which is what anyone typing a name expects.
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        save = true;
                    }
                    if let Some(err) = &self.error {
                        ui.label(RichText::new(err).size(11.5).color(RED));
                    }
                });
            });

            ui.add_space(10.0);
            side_head(ui, "FACE");
            ui.horizontal_wrapped(|ui| {
                for avatar in AVATARS {
                    if pick_button(ui, avatar, draft.avatar == avatar, ACCENT) {
                        draft.avatar = avatar.into();
                    }
                }
            });

            ui.add_space(10.0);
            side_head(ui, "COLOUR");
            ui.horizontal_wrapped(|ui| {
                for hex in COLORS {
                    if swatch(ui, parse_color(hex), draft.color == hex) {
                        draft.color = hex.into();
                    }
                }
            });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if apply_btn(ui, if editing { "✔ save" } else { "✔ create" }, true).clicked() {
                    save = true;
                }
                if mini_btn(ui, "cancel").clicked() {
                    close = true;
                }
                if editing {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Deleting takes a profile's whole history with it, so
                        // it asks — once, in place, without a modal.
                        if draft.confirm_delete {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✖ delete for good").size(12.0).color(RED),
                                    )
                                    .fill(tint(RED))
                                    .stroke(Stroke::new(1.0, RED))
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                delete = true;
                            }
                            ui.label(
                                RichText::new("progress, favourites and playlists too")
                                    .size(11.0)
                                    .color(TEXT_DIM),
                            );
                        } else if mini_btn(ui, "✖ delete profile").clicked() {
                            draft.confirm_delete = true;
                        }
                    });
                }
            });
        });

        // Applied after the closure so the borrow on `draft` is over.
        if save {
            let (id, name, avatar, color) = {
                let d = self.draft.as_ref().expect("checked above");
                (d.id, d.name.clone(), d.avatar.clone(), d.color.clone())
            };
            let before = progress.warning.clone();
            let done = match id {
                Some(id) => progress.update_profile(id, &name, &avatar, &color),
                None => progress.create_profile(&name, &avatar, &color).is_some(),
            };
            if done {
                self.draft = None;
                self.error = None;
            } else {
                self.error = progress.warning.clone().or(before);
                progress.warning = None; // it is shown here, not in the status strip
            }
        } else if delete {
            if let Some(id) = self.draft.as_ref().and_then(|d| d.id) {
                progress.delete_profile(id);
            }
            self.draft = None;
            self.manage = !profiles.is_empty();
        } else if close {
            self.draft = None;
            self.error = None;
        }
    }
}

/// One avatar in the face picker.
fn pick_button(ui: &mut Ui, glyph: &str, on: bool, accent: Color32) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(9),
        if on { alpha(accent, 0x3a) } else { PANEL2 },
    );
    p.rect_stroke(
        rect,
        CornerRadius::same(9),
        Stroke::new(1.0, if on { accent } else { BORDER }),
        egui::StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(20.0),
        if on { TEXT } else { TEXT_DIM },
    );
    response.clicked()
}

fn swatch(ui: &mut Ui, color: Color32, on: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 28.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(8), color);
    if on {
        p.rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(10),
            Stroke::new(2.0, TEXT),
            egui::StrokeKind::Outside,
        );
    }
    response.clicked()
}

/// `#rrggbb` from the database, falling back to the app accent rather than to
/// black — a profile whose colour failed to parse should still look deliberate.
pub fn parse_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        }
    }
    ACCENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_parse_from_the_stored_form() {
        assert_eq!(parse_color("#7c6cff"), ACCENT);
        assert_eq!(parse_color("7c6cff"), ACCENT);
        assert_eq!(parse_color("#22d3ee"), ACCENT2);
    }

    #[test]
    fn a_broken_colour_falls_back_instead_of_going_black() {
        assert_eq!(parse_color(""), ACCENT);
        assert_eq!(parse_color("#zzzzzz"), ACCENT);
        assert_eq!(parse_color("#fff"), ACCENT);
    }

    #[test]
    fn every_offered_colour_is_a_colour() {
        // A swatch that silently fell back would look like a duplicate.
        let parsed: Vec<Color32> = COLORS.iter().map(|c| parse_color(c)).collect();
        for (i, a) in parsed.iter().enumerate() {
            for b in parsed.iter().skip(i + 1) {
                assert_ne!(a, b, "two swatches render the same");
            }
        }
    }

    #[test]
    fn the_editor_opens_blank_for_a_new_profile() {
        let mut s = Screen::default();
        assert!(s.draft.is_none());
        s.start_creating();
        let d = s.draft.as_ref().expect("editor is open");
        assert!(d.id.is_none());
        assert!(d.name.is_empty());
        assert!(!d.confirm_delete, "delete is never armed on arrival");
    }
}
