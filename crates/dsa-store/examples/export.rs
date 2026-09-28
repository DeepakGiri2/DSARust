//! Export one profile's progress for the cloud platform's "Import desktop
//! progress" (web dashboard). The output is the `ImportRequest` shape the API
//! accepts at `POST /profiles/{pid}/import`, so moving to the web keeps every
//! tick, star and playlist.
//!
//! ```text
//! cargo run -p dsa-store --example export -- <progress.db> "<profile name>" > progress.json
//! cargo run -p dsa-store --example export -- <progress.db>                    # list profiles
//! ```
//!
//! The database lives in the platform data directory, e.g.
//! `%APPDATA%\dsa\dsa-visualized\data\progress.db` on Windows.

use dsa_store::{Status, Store};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: export <progress.db> [profile name]");
        std::process::exit(2);
    };
    let store = Store::open(&path).unwrap_or_else(|e| {
        eprintln!("cannot open {path}: {e}");
        std::process::exit(1);
    });
    let profiles = store.profiles().expect("profiles");

    let Some(name) = args.next() else {
        eprintln!("profiles in {path}:");
        for p in &profiles {
            let s = store.stats(p.id).expect("stats");
            eprintln!(
                "  {} {:<24} {} solved, {} attempted",
                p.avatar, p.name, s.solved, s.attempted
            );
        }
        return;
    };
    let Some(profile) = profiles.iter().find(|p| p.name.eq_ignore_ascii_case(&name)) else {
        eprintln!("no profile named {name:?}");
        std::process::exit(1);
    };

    let snapshot = store.snapshot(profile.id).expect("snapshot");
    let mut entries = String::new();
    for (i, (slug, e)) in snapshot.iter().enumerate() {
        let status = match e.status {
            Status::Todo => "todo",
            Status::Attempted => "attempted",
            Status::Solved => "solved",
        };
        if i > 0 {
            entries.push(',');
        }
        entries.push_str(&format!(
            "\n    {}: {{\"status\": \"{status}\", \"favourite\": {}, \"attempts\": {}}}",
            json_string(slug),
            e.favourite,
            e.attempts
        ));
    }

    let mut playlists = String::new();
    for (i, pl) in store
        .playlists(profile.id)
        .expect("playlists")
        .iter()
        .enumerate()
    {
        let slugs: Vec<String> = store
            .playlist_slugs(pl.id)
            .expect("playlist items")
            .iter()
            .map(|s| json_string(s))
            .collect();
        if i > 0 {
            playlists.push(',');
        }
        playlists.push_str(&format!(
            "\n    {{\"name\": {}, \"slugs\": [{}]}}",
            json_string(&pl.name),
            slugs.join(", ")
        ));
    }

    println!("{{\n  \"entries\": {{{entries}\n  }},\n  \"playlists\": [{playlists}\n  ]\n}}");
    eprintln!(
        "exported {} problem(s) and their playlists for {:?}",
        snapshot.len(),
        profile.name
    );
}

/// A JSON string literal. This crate has no JSON dependency, and slugs and
/// playlist names are all that need escaping.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
