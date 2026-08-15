//! DSA Visualized — a native NeetCode animation debugger.
//!
//! One binary, one `content/` directory beside it. The same source builds and
//! behaves identically on Windows, Linux and macOS: the GUI is pure Rust
//! (egui/eframe over OpenGL), the scripting engine is pure Rust (Rhai), and
//! the only `#[cfg(windows)]` in the workspace is in the process harness.
//!
//! One C dependency: SQLite, compiled from bundled source rather than linked
//! against the system's, so a machine with no libsqlite3 still runs the binary.
//! It holds the profiles and their progress; see `dsa-store`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assistant;
mod helper;
mod highlight;
mod list;
mod practice;
mod problem;
mod profiles;
mod progress;
mod settings;
mod style;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let content_root =
        match resolve_content() {
            Ok(p) => p,
            Err(tried) => {
                // Without content there is nothing to show, and the reason is
                // almost always "the folder did not travel with the binary".
                let msg =
                    format!(
                "Could not find the content directory.\n\nLooked for catalog.toml in:\n{}\n\n\
                 Put the content/ folder next to the executable, or set DSA_CONTENT.",
                tried.iter().map(|p| format!("  {}", p.display())).collect::<Vec<_>>().join("\n")
            );
                eprintln!("{msg}");
                return show_fatal(&msg);
            }
        };
    log::info!("content root: {}", content_root.display());

    // `--screenshot <path> [--slug <slug>] [--frames N]` renders the app, saves
    // its own window to a PNG and exits. It exists because reviewing a change
    // to the visual language otherwise means asking someone to look at it and
    // describe what they see.
    let shot = app::Shot::from_args(std::env::args().skip(1));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title("DSA Visualized")
            .with_app_id("dsa-visualized"),
        ..Default::default()
    };

    eframe::run_native(
        "DSA Visualized",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc, content_root);
            app.set_shot(shot);
            Ok(Box::new(app))
        }),
    )
}

fn resolve_content() -> Result<PathBuf, Vec<PathBuf>> {
    // The app's own data directory is searched too, so a user can drop an
    // updated or third-party content pack there without touching the install.
    let mut extra = Vec::new();
    if let Some(dirs) = directories::ProjectDirs::from("dev", "dsa", "dsa-visualized") {
        extra.push(dirs.data_dir().join("content"));
    }
    dsa_content::find_content_root(&extra)
}

/// A GUI app that dies before opening a window looks like it did nothing, so
/// the fatal path still shows something on screen.
fn show_fatal(message: &str) -> eframe::Result<()> {
    let text = message.to_string();
    eframe::run_native(
        "DSA Visualized",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 340.0]),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(Fatal { text: text.clone() }) as Box<dyn eframe::App>)),
    )
}

struct Fatal {
    text: String,
}

impl eframe::App for Fatal {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.heading("Content not found");
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&self.text).monospace().size(12.0));
        });
    }
}
