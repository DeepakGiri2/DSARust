//! The problem list — the app's home screen.
//!
//! Laid out to match the web version: hero, tier segments, search,
//! interactive-only filter and a count, then one section per category with a
//! 📘 guide button and a row per problem. Interactive problems open the
//! debugger; the rest link out to LeetCode, exactly as before.

use crate::profiles::parse_color;
use crate::progress::{Progress, StatusFilter};
use crate::settings::Settings;
use crate::style::*;
use dsa_content::Library;
use dsa_core::problem::Tier;
use dsa_store::Status;
use egui::{Align, Layout, RichText, ScrollArea, Stroke, Ui};

/// What the list wants the app to do next.
pub enum ListAction {
    None,
    Open(String),
    OpenGuide(String),
    OpenExternal(String),
    /// Back to "who's practising?".
    SwitchProfile,
}

pub fn show(
    ui: &mut Ui,
    lib: &Library,
    settings: &mut Settings,
    progress: &mut Progress,
    search: &mut String,
) -> ListAction {
    let mut action = ListAction::None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(28.0);
            // Centre the page on wide screens the way a max-width container does.
            let width = ui.available_width().min(1180.0);
            let pad = ((ui.available_width() - width) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.vertical(|ui| {
                    ui.set_max_width(width);
                    if let Some(a) = hero(ui, lib, settings, progress, search) {
                        action = a;
                    }
                    ui.add_space(18.0);
                    if let Some(a) = categories(ui, lib, settings, progress, search) {
                        action = a;
                    }
                    ui.add_space(24.0);
                    footer(ui);
                    ui.add_space(30.0);
                });
            });
        });

    action
}

fn hero(
    ui: &mut Ui,
    lib: &Library,
    settings: &mut Settings,
    progress: &mut Progress,
    search: &mut String,
) -> Option<ListAction> {
    let animated = lib.catalog.iter().filter(|c| c.viz).count();
    let mut action = None;

    // A kicker above the title: the three numbers someone actually wants, in
    // the smallest form that still reads as a claim rather than decoration.
    ui.horizontal(|ui| {
        pill(ui, "▶ step-through debugger", ACCENT2, tint(ACCENT2));
        pill(
            ui,
            &format!("{animated} animated · {} categories", lib.categories.len()),
            TEXT_DIM,
            PANEL2,
        );
        pill(ui, "Go · C++ · Java", ACCENT, tint(ACCENT));

        // Who is looking, and their score — the one place the profile is always
        // visible, and the way back to the picker.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(profile) = progress.current() {
                let accent = parse_color(&profile.color);
                let stats = progress.stats();
                let total = lib.catalog.len().max(1);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!(
                                "{} {} · {}/{} solved",
                                profile.avatar, profile.name, stats.solved, total
                            ))
                            .size(12.0)
                            .color(TEXT),
                        )
                        .fill(alpha(accent, 0x2e))
                        .stroke(Stroke::new(1.0, alpha(accent, 0xaa)))
                        .corner_radius(egui::CornerRadius::same(255)),
                    )
                    .on_hover_text("Switch profile")
                    .clicked()
                {
                    action = Some(ListAction::SwitchProfile);
                }
            }
        });
    });
    ui.add_space(10.0);

    gradient_heading(ui, "DSA ", "Visualized", 48.0);
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let dim = |ui: &mut Ui, t: &str| ui.label(RichText::new(t).size(13.5).color(TEXT_DIM));
        dim(ui, "The NeetCode roadmaps, plus the interview extras they leave out. Every variable, map and pointer,");
        ui.label(RichText::new("one step at a time").size(13.5).strong().color(TEXT));
        dim(ui, ".");
    });

    ui.add_space(16.0);

    // The controls sit on their own raised surface so they stop reading as more
    // page text and start reading as the toolbar they are.
    egui::Frame::default()
        .fill(alpha(PANEL, 0xd8))
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut tier = settings.tier;
                let labels: Vec<(Tier, String)> = Tier::ALL
                    .iter()
                    .map(|t| (*t, t.title().to_string()))
                    .collect();
                let opts: Vec<(Tier, &str)> =
                    labels.iter().map(|(t, s)| (*t, s.as_str())).collect();
                if seg(ui, &mut tier, &opts) {
                    settings.tier = tier;
                }

                ui.add(
                    egui::TextEdit::singleline(search)
                        .hint_text("search problems…")
                        .desired_width(220.0),
                );

                ui.checkbox(
                    &mut settings.viz_only,
                    RichText::new("interactive only").size(13.0),
                );

                if toggle_btn(ui, "✨", settings.backdrop)
                    .on_hover_text(
                        "Animated background — turn it off to stop the app repainting when idle",
                    )
                    .clicked()
                {
                    settings.backdrop = !settings.backdrop;
                }

                let shown = visible(lib, settings, progress, search).count();
                pill(ui, &format!("{shown} shown"), ACCENT2, tint(ACCENT2));
            });

            // Progress filters live on their own line: they belong to whoever
            // is signed in, where the row above belongs to the catalogue.
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let mut status = settings.status_filter;
                let opts: Vec<(StatusFilter, &str)> =
                    StatusFilter::ALL.iter().map(|f| (*f, f.label())).collect();
                if seg(ui, &mut status, &opts) {
                    settings.status_filter = status;
                }

                if toggle_btn(ui, "⭐ favourites", settings.favourites_only)
                    .on_hover_text("Only problems you have starred")
                    .clicked()
                {
                    settings.favourites_only = !settings.favourites_only;
                }

                playlist_picker(ui, settings, progress);

                let stats = progress.stats();
                if stats.solved > 0 || stats.attempted > 0 {
                    pill(ui, &format!("✔ {}", stats.solved), GREEN, tint(GREEN));
                    if stats.attempted > 0 {
                        pill(ui, &format!("◐ {}", stats.attempted), AMBER, tint(AMBER));
                    }
                }
            });
        });

    action
}

/// The playlist filter, plus the only place a playlist is created or deleted.
fn playlist_picker(ui: &mut Ui, settings: &mut Settings, progress: &mut Progress) {
    let current = settings
        .playlist
        .and_then(|id| progress.playlist(id))
        .map(|p| format!("♪ {} ({})", p.name, p.len))
        .unwrap_or_else(|| "♪ all problems".to_string());

    let mut create = false;
    let mut delete = None;
    egui::ComboBox::from_id_salt("playlist-filter")
        .selected_text(RichText::new(current).size(12.0))
        .width(190.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(settings.playlist.is_none(), "all problems")
                .clicked()
            {
                settings.playlist = None;
            }
            for list in progress.playlists() {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            settings.playlist == Some(list.id),
                            format!("{} ({})", list.name, list.len),
                        )
                        .clicked()
                    {
                        settings.playlist = Some(list.id);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button("✖")
                            .on_hover_text("Delete this playlist")
                            .clicked()
                        {
                            delete = Some(list.id);
                        }
                    });
                });
            }
            ui.separator();
            if ui.button("+ new playlist").clicked() {
                create = true;
            }
        });

    if let Some(id) = delete {
        progress.delete_playlist(id);
        if settings.playlist == Some(id) {
            settings.playlist = None;
        }
    }
    if create {
        // Named after the next free number rather than opening a dialog: the
        // name is editable nowhere else yet, and an unnamed list is useless.
        let n = progress.playlists().len() + 1;
        let mut name = format!("playlist {n}");
        let mut bump = n;
        while progress.playlists().iter().any(|p| p.name == name) {
            bump += 1;
            name = format!("playlist {bump}");
        }
        if let Some(id) = progress.create_playlist(&name) {
            settings.playlist = Some(id);
        }
    }
}

/// Catalog rows passing every filter: the catalogue's own, and the profile's.
fn visible<'a>(
    lib: &'a Library,
    settings: &'a Settings,
    progress: &'a Progress,
    search: &'a str,
) -> impl Iterator<Item = &'a dsa_core::problem::CatalogItem> + 'a {
    let needle = search.trim().to_lowercase();
    lib.catalog.iter().filter(move |c| {
        settings.tier.contains(c.tier)
            && (!settings.viz_only || c.viz)
            && (needle.is_empty() || c.title.to_lowercase().contains(&needle))
            && progress.accepts(
                &c.slug,
                settings.status_filter,
                settings.favourites_only,
                settings.playlist,
            )
    })
}

fn categories(
    ui: &mut Ui,
    lib: &Library,
    settings: &Settings,
    progress: &mut Progress,
    search: &str,
) -> Option<ListAction> {
    let mut action = None;
    // Collected up front: the row loop mutates `progress` (a star, a tick),
    // which cannot happen while an iterator is still borrowing it.
    let mut wanted: Vec<(String, Vec<dsa_core::problem::CatalogItem>)> = Vec::new();
    for cat in &lib.categories {
        let rows: Vec<dsa_core::problem::CatalogItem> = visible(lib, settings, progress, search)
            .filter(|c| &c.category == cat)
            .cloned()
            .collect();
        if !rows.is_empty() {
            wanted.push((cat.clone(), rows));
        }
    }

    for (cat, rows) in &wanted {
        let cat = cat.as_str();

        ui.add_space(18.0);
        let head = ui.horizontal(|ui| {
            section_head(ui, cat, rows.len());
            if mini_btn(ui, "📘 guide")
                .on_hover_text(format!(
                    "How the structures and techniques behind “{cat}” work — syntax, complexity, pitfalls"
                ))
                .clicked()
            {
                action = Some(ListAction::OpenGuide(cat.to_string()));
            }
            ui.cursor().min.x
        });
        head_rule(ui, head.inner, head.response.rect.center().y);
        ui.add_space(8.0);

        for item in rows {
            let status = progress.status(&item.slug);
            let favourite = progress.is_favourite(&item.slug);
            // A solved problem is edged in green whatever its difficulty: down
            // a long list, "what is left" is the question being asked.
            let edge = match (status, item.viz) {
                (Status::Solved, _) => GREEN,
                (Status::Attempted, _) => AMBER,
                (_, true) => difficulty_color(item.difficulty),
                (_, false) => BORDER,
            };

            // Clicks on the star must not also open the problem.
            let mut inner_click = false;
            let resp = row_frame(ui, edge, |ui| {
                let (glyph, color) = match status {
                    Status::Solved => ("✔", GREEN),
                    Status::Attempted => ("◐", AMBER),
                    Status::Todo if item.viz => ("▶", ACCENT),
                    Status::Todo => ("↗", TEXT_DIM),
                };
                ui.label(RichText::new(glyph).size(11.0).color(color));
                ui.label(RichText::new(&item.title).size(13.5).color(
                    if status == Status::Solved {
                        alpha(TEXT, 0xbb) // done, so it recedes
                    } else if item.viz {
                        TEXT
                    } else {
                        TEXT_DIM
                    },
                ));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if star(ui, favourite).clicked() {
                        inner_click = true;
                    }
                    diff_pill(ui, item.difficulty);
                    if item.viz {
                        pill(ui, "interactive · 3 langs", ACCENT2, tint(ACCENT2));
                    }
                });
            });

            if inner_click {
                progress.toggle_favourite(&item.slug);
            } else if resp.clicked() {
                action = Some(if item.viz {
                    ListAction::Open(item.slug.clone())
                } else {
                    ListAction::OpenExternal(item.leetcode_url())
                });
            }
            if !item.viz {
                resp.on_hover_text("Opens on LeetCode — visualization not built yet");
            }
            ui.add_space(5.0);
        }
    }

    action
}

/// The favourite toggle, filled when on and a faint outline when off — visible
/// enough to be discoverable, quiet enough not to compete with the title.
fn star(ui: &mut Ui, on: bool) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(if on { "⭐" } else { "☆" })
                .size(13.0)
                .color(if on { AMBER } else { alpha(TEXT_DIM, 0x99) }),
        )
        .fill(egui::Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .frame(false),
    )
    .on_hover_text(if on {
        "Remove from favourites"
    } else {
        "Add to favourites"
    })
}

fn footer(ui: &mut Ui) {
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Problem lists follow neetcode.io roadmaps (the “250” tier is an extended superset of the 150); \
             “Interview Extra” adds high-frequency problems and patterns the roadmap leaves out. \
             Practice runs execute on a local toolchain when one is installed, otherwise on the Compiler Explorer (godbolt.org) API.",
        )
        .size(11.5)
        .color(TEXT_DIM),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::problem::{CatalogItem, Difficulty};

    fn item(title: &str, tier: Tier, viz: bool) -> CatalogItem {
        CatalogItem {
            slug: title.to_lowercase().replace(' ', "-"),
            title: title.into(),
            category: "Arrays & Hashing".into(),
            difficulty: Difficulty::Easy,
            tier,
            leetcode: None,
            viz,
        }
    }

    /// `visible` is the filter the header count and every section share, so it
    /// is worth testing without a window. Driving the real function through a
    /// throwaway `Library` keeps this from being a second copy of the rules
    /// that can quietly disagree with the one on screen.
    fn filtered(items: Vec<CatalogItem>, settings: &Settings, search: &str) -> Vec<String> {
        filtered_for(items, settings, search, &Progress::in_memory())
    }

    fn filtered_for(
        items: Vec<CatalogItem>,
        settings: &Settings,
        search: &str,
        progress: &Progress,
    ) -> Vec<String> {
        let mut lib = Library::load(std::env::temp_dir().join("dsa-list-tests-no-content"));
        lib.catalog = items;
        visible(&lib, settings, progress, search)
            .map(|c| c.title.clone())
            .collect()
    }

    #[test]
    fn the_tier_filter_nests() {
        let items = vec![
            item("Two Sum", Tier::T50, true),
            item("Valid Sudoku", Tier::T150, false),
            item("Sort Colors", Tier::T250, false),
        ];
        let mut s = Settings {
            tier: Tier::T50,
            ..Default::default()
        };
        assert_eq!(filtered(items.clone(), &s, ""), vec!["Two Sum"]);
        s.tier = Tier::T150;
        assert_eq!(filtered(items.clone(), &s, "").len(), 2);
        s.tier = Tier::T250;
        assert_eq!(filtered(items, &s, "").len(), 3);
    }

    #[test]
    fn interactive_only_hides_unbuilt_problems() {
        let items = vec![
            item("Two Sum", Tier::T50, true),
            item("Sort Colors", Tier::T50, false),
        ];
        let s = Settings {
            tier: Tier::T250,
            viz_only: true,
            ..Default::default()
        };
        assert_eq!(filtered(items, &s, ""), vec!["Two Sum"]);
    }

    #[test]
    fn search_is_case_insensitive_and_matches_anywhere() {
        let items = vec![
            item("Two Sum", Tier::T50, true),
            item("Group Anagrams", Tier::T50, true),
        ];
        let s = Settings {
            tier: Tier::T250,
            ..Default::default()
        };
        assert_eq!(filtered(items.clone(), &s, "SUM"), vec!["Two Sum"]);
        assert_eq!(
            filtered(items.clone(), &s, " anagram "),
            vec!["Group Anagrams"]
        );
        assert_eq!(filtered(items, &s, "zzz").len(), 0);
    }

    /// A profile with Two Sum solved and Group Anagrams starred.
    fn progress_with_history() -> Progress {
        let mut p = Progress::in_memory();
        let me = p.create_profile("Test", "🎓", "#7c6cff").unwrap();
        p.enter(me.id);
        p.mark_solved("two-sum");
        p.toggle_favourite("group-anagrams");
        p
    }

    fn three() -> Vec<CatalogItem> {
        vec![
            item("Two Sum", Tier::T50, true),
            item("Group Anagrams", Tier::T50, true),
            item("Valid Sudoku", Tier::T50, true),
        ]
    }

    #[test]
    fn the_progress_filter_narrows_the_same_list_the_count_reports() {
        let p = progress_with_history();
        let mut s = Settings {
            tier: Tier::T250,
            ..Default::default()
        };

        s.status_filter = StatusFilter::Solved;
        assert_eq!(filtered_for(three(), &s, "", &p), vec!["Two Sum"]);

        s.status_filter = StatusFilter::Todo;
        assert_eq!(
            filtered_for(three(), &s, "", &p),
            vec!["Group Anagrams", "Valid Sudoku"]
        );

        s.status_filter = StatusFilter::All;
        assert_eq!(filtered_for(three(), &s, "", &p).len(), 3);
    }

    #[test]
    fn favourites_stack_with_search_and_tier() {
        let p = progress_with_history();
        let s = Settings {
            tier: Tier::T250,
            favourites_only: true,
            ..Default::default()
        };
        assert_eq!(filtered_for(three(), &s, "", &p), vec!["Group Anagrams"]);
        assert_eq!(filtered_for(three(), &s, "sudoku", &p).len(), 0);
    }

    #[test]
    fn without_a_profile_nothing_is_filtered_away() {
        // The catalogue has to stay usable before anyone has signed in.
        let p = Progress::in_memory();
        let s = Settings {
            tier: Tier::T250,
            ..Default::default()
        };
        assert_eq!(filtered_for(three(), &s, "", &p).len(), 3);
    }
}
