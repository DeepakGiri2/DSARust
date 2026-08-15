//! Application shell and routing.
//!
//! Two screens, as in the web version: the problem list, and a problem page
//! with a Practice tab and a Visualize tab. The helper is a modal over either.

use crate::helper::Helper;
use crate::list::{self, ListAction};
use crate::practice::Practice;
use crate::problem::Visualize;
use crate::profiles::{self, ProfileAction};
use crate::progress::Progress;
use crate::settings::Settings;
use crate::style::*;
use dsa_content::{Change, ContentWatcher, Library};
use dsa_core::problem::LangId;
use dsa_harness::Harness;
use dsa_store::Status;
use dsa_viz::Theme;
use egui::{Align, Context, Layout, RichText, Ui};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

/// Where progress lives: the user's own data directory, not next to the binary.
/// Content travels with the install; a profile's history does not, and a
/// read-only install directory must not cost someone their history.
pub fn progress_db() -> PathBuf {
    directories::ProjectDirs::from("dev", "dsa", "dsa-visualized")
        .map(|d| d.data_dir().join("progress.db"))
        .unwrap_or_else(|| PathBuf::from("progress.db"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Practice,
    Visualize,
}

enum Route {
    /// "Who's practising?" — the way in, and the way to switch.
    Profiles,
    List,
    Problem(String),
}

pub struct App {
    lib: Library,
    watcher: Option<ContentWatcher>,
    harness: Harness,
    theme: Theme,
    pub settings: Settings,
    progress: Progress,

    route: Route,
    mode: Mode,
    search: String,
    helper: Helper,
    practice: Practice,
    viz: Visualize,
    picker: profiles::Screen,

    last_frame: Instant,
    reload_note: Option<(String, Instant)>,
    shot: Option<Shot>,
}

/// A one-shot "render, save a PNG of my own window, quit" job.
///
/// Only the app's own framebuffer is captured — nothing else on the desktop —
/// which is what makes this safe to run unattended.
pub struct Shot {
    path: PathBuf,
    /// Which problem to open first, if any.
    slug: Option<String>,
    /// Which profile to sign in as, by name. Without it the capture is of the
    /// picker, since that is where the app starts.
    profile: Option<String>,
    /// Capture the ⏵ Visualize tab rather than ✏ Practice.
    visualize: bool,
    /// Frames to render before asking for the capture. Fonts, layout and the
    /// first trace all settle over the first few frames; capturing frame 0
    /// photographs a half-built window.
    warmup: u32,
    asked: bool,
}

impl Shot {
    pub fn from_args(args: impl Iterator<Item = String>) -> Option<Self> {
        let args: Vec<String> = args.collect();
        let value = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let path = value("--screenshot")?;
        Some(Self {
            path: PathBuf::from(path),
            slug: value("--slug"),
            profile: value("--profile"),
            visualize: args.iter().any(|a| a == "--visualize"),
            warmup: value("--frames").and_then(|f| f.parse().ok()).unwrap_or(30),
            asked: false,
        })
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, content_root: PathBuf) -> Self {
        let mut settings = Settings::load(cc.storage);
        let lib = Library::load(content_root.clone());
        let watcher = match ContentWatcher::new(&content_root) {
            Ok(w) => Some(w),
            Err(e) => {
                log::warn!("hot reload unavailable: {e}");
                None
            }
        };
        let harness = Harness::detect(&lib.languages);
        install(&cc.egui_ctx);
        // egui's default point size is tuned for a laptop panel; on a large
        // monitor at 100% scaling the whole app comes out microscopic. This is
        // the user's own zoom (ctrl +/- and ctrl+scroll change it), so it is
        // read back and remembered rather than forced every frame.
        cc.egui_ctx.set_zoom_factor(settings.zoom.clamp(0.7, 3.0));

        let mut practice = Practice::default();
        practice
            .assistant
            .set_prompts(dsa_ai::Prompts::load(&content_root));

        let mut progress = Progress::open(&progress_db());
        // Content is editable at runtime, so a slug can be renamed out from
        // under a profile's history. Drop what no longer exists, but only when
        // the catalogue actually loaded — see `Progress::prune`.
        let known: BTreeSet<String> = lib.catalog.iter().map(|c| c.slug.clone()).collect();
        progress.prune(&known);

        // Remembered profile, if it still exists. A single-user machine should
        // not be asked "who's practising?" every launch; the chip on the list
        // is one click from the picker.
        let entered = settings.profile.is_some_and(|id| progress.enter(id));
        if !entered {
            settings.profile = None;
        }

        let mut app = Self {
            theme: Theme::dark(),
            route: if entered {
                Route::List
            } else {
                Route::Profiles
            },
            mode: Mode::Practice,
            search: String::new(),
            helper: Helper::default(),
            practice,
            viz: Visualize::default(),
            picker: profiles::Screen::default(),
            last_frame: Instant::now(),
            reload_note: None,
            shot: None,
            lib,
            watcher,
            harness,
            settings,
            progress,
        };
        // First run: there is nobody to pick, so go straight to making someone.
        if !entered && app.progress.profiles().is_empty() {
            app.picker.start_creating();
        }
        if entered {
            if let Some(slug) = app.settings.selected.clone() {
                if app.lib.pack(&slug).is_some() {
                    app.open(&slug);
                }
            }
        }
        app
    }

    /// Enter a profile and go to the catalogue.
    fn enter_profile(&mut self, id: i64) {
        if self.progress.enter(id) {
            self.settings.profile = Some(id);
            // Filters belong to whoever set them; a different profile's
            // playlist id would otherwise hide their whole catalogue.
            self.settings.playlist = None;
            self.route = Route::List;
        }
    }

    fn switch_profile(&mut self) {
        self.progress.leave();
        self.settings.profile = None;
        self.settings.selected = None;
        self.route = Route::Profiles;
    }

    /// Arm the `--screenshot` job, opening the requested problem if one was
    /// named so the capture can be of a screen other than the list.
    pub fn set_shot(&mut self, shot: Option<Shot>) {
        if let Some(shot) = &shot {
            if let Some(name) = &shot.profile {
                match self
                    .progress
                    .profiles()
                    .into_iter()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    Some(p) => self.enter_profile(p.id),
                    None => log::warn!("--profile {name}: no such profile"),
                }
            }
            match shot.slug.clone() {
                Some(slug) if self.lib.pack(&slug).is_some() => self.open(&slug),
                Some(slug) => log::warn!("--slug {slug}: no such problem"),
                None => self.back(),
            }
            if shot.visualize {
                self.mode = Mode::Visualize;
                // Skip the "try it yourself first" gate: the point of the
                // capture is the animation behind it.
                self.viz.revealed = true;
            }
        }
        self.shot = shot;
    }

    /// Drive the screenshot job: settle, ask, save, quit.
    fn run_shot(&mut self, ctx: &Context) {
        let Some(shot) = &mut self.shot else {
            return;
        };
        if shot.warmup > 0 {
            shot.warmup -= 1;
            ctx.request_repaint();
            return;
        }
        if !shot.asked {
            shot.asked = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ctx.request_repaint();
            return;
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            ctx.request_repaint();
            return;
        };
        let shot = self.shot.take().expect("checked above");
        let [w, h] = [image.width() as u32, image.height() as u32];
        let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
        match image::RgbaImage::from_raw(w, h, bytes) {
            Some(buf) => match buf.save(&shot.path) {
                Ok(()) => log::info!("screenshot: {} ({w}×{h})", shot.path.display()),
                Err(e) => log::error!("screenshot: {e}"),
            },
            None => log::error!("screenshot: unexpected buffer size"),
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn languages(&self) -> Vec<(LangId, String)> {
        self.lib
            .languages
            .iter()
            .map(|l| (l.id.clone(), l.label.clone()))
            .collect()
    }

    fn open(&mut self, slug: &str) {
        self.route = Route::Problem(slug.to_string());
        self.settings.selected = Some(slug.to_string());
        // Land on Practice, exactly as the web version does.
        self.mode = Mode::Practice;
        self.practice = {
            let mut p = Practice::default();
            p.assistant
                .set_prompts(dsa_ai::Prompts::load(&self.lib.root));
            p
        };
        if let Some(pack) = self.lib.pack(slug) {
            if pack.source(&self.settings.lang).is_none() {
                // First in manifest order, so the fallback is the same language
                // the leftmost tab shows.
                if let Some(first) = self
                    .lib
                    .languages
                    .iter()
                    .find(|l| pack.source(&l.id).is_some())
                {
                    self.settings.lang = first.id.clone();
                }
            }
        }
        let settings = self.settings.clone();
        self.viz.load(&self.lib, slug, &settings);
    }

    fn back(&mut self) {
        self.settings.selected = None;
        // There is no catalogue without someone to own the progress on it, so
        // "back" from a problem lands on the picker when nobody is signed in.
        self.route = if self.progress.profile_id().is_some() {
            Route::List
        } else {
            Route::Profiles
        };
    }

    fn tick(&mut self, ctx: &Context) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        if self.viz.tick(dt) {
            ctx.request_repaint();
        }
        // A test sweep that comes back green ticks the problem off, so the poll
        // needs to know which problem is open and how many cases it has.
        let (slug, cases) = match &self.route {
            Route::Problem(slug) => {
                let n = self.lib.pack(slug).map_or(0, |p| p.meta.tests.len());
                (slug.clone(), n)
            }
            _ => (String::new(), 0),
        };
        let mut settings = std::mem::take(&mut self.settings);
        if self
            .practice
            .poll(&mut settings, &mut self.progress, &slug, cases)
        {
            ctx.request_repaint();
        }
        self.settings = settings;

        self.poll_content(ctx);
    }

    fn poll_content(&mut self, ctx: &Context) {
        let Some(watcher) = &self.watcher else { return };
        let changes = watcher.poll();
        if changes.is_empty() {
            return;
        }
        let wide = changes
            .iter()
            .any(|c| matches!(c, Change::Catalog | Change::Languages | Change::Prelude));
        if wide {
            self.lib.reload_all();
            self.reload_note = Some(("content reloaded".into(), Instant::now()));
        } else {
            for c in &changes {
                if let Change::Pack(slug) = c {
                    self.lib.load_pack(slug);
                    self.reload_note = Some((format!("reloaded {slug}"), Instant::now()));
                }
            }
        }
        if let Route::Problem(slug) = &self.route {
            let slug = slug.clone();
            let settings = self.settings.clone();
            // Keep the viewer's position: re-authoring an animation should not
            // throw you back to step 0.
            self.viz.retrace(&self.lib, &slug, &settings, false);
        }
        ctx.request_repaint();
    }

    // ── chrome ──────────────────────────────────────────────────────────────

    fn problem_header(&mut self, ui: &mut Ui, slug: &str) {
        let Some(pack) = self.lib.pack(slug) else {
            return;
        };
        let (title, category, difficulty, complexity, url) = (
            pack.meta.title.clone(),
            pack.meta.category.clone(),
            pack.meta.difficulty,
            pack.meta.complexity.clone(),
            pack.meta.leetcode_url(),
        );
        // Driven from `languages.toml`, not from the pack: its sources are a
        // map keyed by id, so asking it gives C++ before Go before Java. The
        // manifest carries the order the tabs are meant to read in.
        let langs: Vec<(LangId, String)> = self
            .lib
            .languages
            .iter()
            .filter(|l| pack.source(&l.id).is_some())
            .map(|l| (l.id.clone(), l.label.clone()))
            .collect();

        toolbar(ui, |ui| {
            if mini_btn(ui, "← list")
                .on_hover_text("Back to the problem list")
                .clicked()
            {
                self.back();
            }
            ui.label(RichText::new(&title).size(17.0).strong().color(TEXT));
            diff_pill(ui, difficulty);
            ui.label(RichText::new(&category).size(11.5).color(TEXT_DIM));
            if !complexity.is_empty() {
                ui.label(
                    RichText::new(&complexity)
                        .monospace()
                        .size(11.0)
                        .color(ACCENT2),
                );
            }
            ui.hyperlink_to(RichText::new("leetcode ↗").size(11.0).color(TEXT_DIM), url);

            // Progress for this problem, where you are looking at the problem.
            let entry = self.progress.entry(slug);
            if entry.attempts > 0 {
                ui.label(
                    RichText::new(format!(
                        "{} attempt{}",
                        entry.attempts,
                        if entry.attempts == 1 { "" } else { "s" }
                    ))
                    .size(11.0)
                    .color(TEXT_DIM),
                )
                .on_hover_text("How many times you have pressed Run or the tests on this");
            }
            let solved = entry.status == Status::Solved;
            if toggle_btn(
                ui,
                if solved {
                    "✔ solved"
                } else {
                    "✔ mark solved"
                },
                solved,
            )
            .on_hover_text(if solved {
                "Solved — click to put it back on the list"
            } else {
                "Tick it off by hand. Passing every test does this for you."
            })
            .clicked()
            {
                self.progress.toggle_solved(slug);
            }
            let favourite = self.progress.is_favourite(slug);
            if toggle_btn(ui, if favourite { "⭐" } else { "☆" }, favourite)
                .on_hover_text("Favourite")
                .clicked()
            {
                self.progress.toggle_favourite(slug);
            }
            self.playlist_menu(ui, slug);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mini_btn(ui, "📘 helper")
                    .on_hover_text(
                        "Data structures & techniques for this category, complexity tables and a \
                         cross-language syntax cheat sheet",
                    )
                    .clicked()
                {
                    let lang = self.settings.lang.clone();
                    self.helper.open(&self.lib.guide, &category, &lang);
                }
                let mut mode = self.mode;
                if seg(
                    ui,
                    &mut mode,
                    &[
                        (Mode::Practice, "✏ Practice"),
                        (Mode::Visualize, "⏵ Visualize"),
                    ],
                ) {
                    self.mode = mode;
                }
                let opts: Vec<(LangId, &str)> = langs
                    .iter()
                    .map(|(id, label)| (id.clone(), label.as_str()))
                    .collect();
                let mut lang = self.settings.lang.clone();
                if !opts.is_empty() && seg(ui, &mut lang, &opts) {
                    self.settings.lang = lang;
                }
            });
        });
        ui.add_space(2.0);
    }

    /// "add to playlist", as a menu of tick-boxes — the same list can hold the
    /// problem or not, and one click either way.
    fn playlist_menu(&mut self, ui: &mut Ui, slug: &str) {
        let lists: Vec<(i64, String, bool)> = self
            .progress
            .playlists()
            .iter()
            .map(|p| {
                (
                    p.id,
                    format!("{} ({})", p.name, p.len),
                    self.progress.playlist_contains(p.id, slug),
                )
            })
            .collect();
        let in_any = lists.iter().any(|(_, _, member)| *member);

        let mut toggle = None;
        let mut create = false;
        egui::containers::menu::MenuButton::new(
            RichText::new(if in_any {
                "♪ in a playlist"
            } else {
                "♪ playlist"
            })
            .size(12.0)
            .color(if in_any { TEXT } else { TEXT_DIM }),
        )
        .ui(ui, |ui| {
            if lists.is_empty() {
                ui.label(RichText::new("no playlists yet").size(11.5).color(TEXT_DIM));
            }
            for (id, label, mut member) in lists {
                if ui.checkbox(&mut member, label).changed() {
                    toggle = Some(id);
                }
            }
            ui.separator();
            if ui.button("+ new playlist with this").clicked() {
                create = true;
                ui.close();
            }
        });

        if let Some(id) = toggle {
            self.progress.toggle_in_playlist(id, slug);
        }
        if create {
            let n = self.progress.playlists().len() + 1;
            let mut name = format!("playlist {n}");
            let mut bump = n;
            while self.progress.playlists().iter().any(|p| p.name == name) {
                bump += 1;
                name = format!("playlist {bump}");
            }
            if let Some(id) = self.progress.create_playlist(&name) {
                self.progress.toggle_in_playlist(id, slug);
            }
        }
    }

    fn status_strip(&mut self, ui: &mut Ui) {
        if let Some((text, at)) = self.reload_note.clone() {
            if at.elapsed().as_secs_f32() < 3.0 {
                ui.label(RichText::new(text).size(11.0).color(GREEN));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(400));
            } else {
                self.reload_note = None;
            }
        }
        if let Some(warning) = self.progress.warning.clone() {
            // Losing your history silently would be the worst kind of failure,
            // so a database that is not the one on disk says so on every screen.
            if ui
                .label(RichText::new("⚠ progress").size(11.0).color(AMBER))
                .on_hover_text(&warning)
                .clicked()
            {
                self.progress.warning = None;
            }
        }
        if !self.lib.errors.is_empty() {
            ui.label(
                RichText::new(format!("{} content error(s)", self.lib.errors.len()))
                    .size(11.0)
                    .color(RED),
            )
            .on_hover_text(
                self.lib
                    .errors
                    .iter()
                    .take(12)
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
}

impl eframe::App for App {
    /// The panels are translucent so the backdrop shows through them, which
    /// makes eframe's default — clear to `panel_fill` — a translucent window.
    /// The window itself is opaque; only the panels above it are not.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        crate::style::BG.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);

        // The invariant every other screen relies on: a star, a tick or a
        // playlist needs a profile to belong to. Deleting the one you were
        // using is the way this is reached.
        if self.progress.profile_id().is_none() && !matches!(self.route, Route::Profiles) {
            self.route = Route::Profiles;
        }

        // Before every panel, so all of them are drawn over it. The lights only
        // drift on the home screen: the debugger has its own moving parts and
        // does not need competition, and a permanently repainting window is a
        // real cost on a laptop. Elsewhere they are simply frozen.
        if (ctx.zoom_factor() - self.settings.zoom).abs() > 0.001 {
            self.settings.zoom = ctx.zoom_factor();
        }

        let drifting = self.settings.backdrop && matches!(self.route, Route::List);
        crate::style::backdrop(ctx, drifting);
        if drifting {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }

        let langs = self.languages();
        let guide = std::mem::take(&mut self.lib.guide);
        self.helper.show(ctx, &guide, &langs);
        self.lib.guide = guide;

        egui::CentralPanel::default().show(ctx, |ui| match &self.route {
            Route::Profiles => {
                if let ProfileAction::Enter(id) = self.picker.show(ui, &mut self.progress) {
                    self.enter_profile(id);
                }
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    self.status_strip(ui)
                });
            }
            Route::List => {
                // Both buffers are moved out and back so the list can hold
                // `&mut` on them while `&self.lib` stays borrowed.
                let mut settings = std::mem::take(&mut self.settings);
                let mut search = std::mem::take(&mut self.search);
                let action = list::show(
                    ui,
                    &self.lib,
                    &mut settings,
                    &mut self.progress,
                    &mut search,
                );
                self.settings = settings;
                self.search = search;
                match action {
                    ListAction::Open(slug) => self.open(&slug),
                    ListAction::OpenGuide(cat) => {
                        let lang = self.settings.lang.clone();
                        self.helper.open(&self.lib.guide, &cat, &lang);
                    }
                    ListAction::OpenExternal(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
                    ListAction::SwitchProfile => self.switch_profile(),
                    ListAction::None => {}
                }
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    self.status_strip(ui)
                });
            }
            Route::Problem(slug) => {
                let slug = slug.clone();
                self.problem_header(ui, &slug);

                if !self.helper.is_open() && self.mode == Mode::Visualize {
                    self.viz.keys(ctx);
                }

                match self.mode {
                    Mode::Practice => {
                        let mut settings = std::mem::take(&mut self.settings);
                        let lang = settings.lang.clone();
                        self.practice.ui(
                            ui,
                            &self.lib,
                            &self.harness,
                            &mut settings,
                            &mut self.progress,
                            &slug,
                            &lang,
                        );
                        self.settings = settings;
                    }
                    Mode::Visualize => {
                        if !self.viz.revealed {
                            let label = self
                                .lib
                                .language(&self.settings.lang)
                                .map(|l| l.label.clone())
                                .unwrap_or_default();
                            if self.viz.gate(ui, &label) {
                                self.mode = Mode::Practice;
                            }
                        } else {
                            let mut settings = std::mem::take(&mut self.settings);
                            let theme = self.theme.clone();
                            let lang = settings.lang.clone();
                            self.viz
                                .ui(ui, &self.lib, &mut settings, &theme, &slug, &lang);
                            self.settings = settings;
                        }
                    }
                }
            }
        });

        self.run_shot(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.store(storage);
    }
}
