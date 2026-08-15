//! Everything the app remembers between runs.
//!
//! Kept in one serializable struct rather than scattered across the app state,
//! so persistence is a single load/store and adding a remembered preference
//! cannot accidentally break the saved format of the others.

use crate::progress::StatusFilter;
use dsa_core::problem::Tier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub dark: bool,
    pub lang: String,
    pub tier: Tier,
    pub selected: Option<String>,
    pub speed: f32,
    pub animate: bool,
    /// Whose progress is on screen. Remembered so a single-user machine is not
    /// asked "who's practising?" every launch; the picker is a click away.
    ///
    /// Progress itself lives in the database, not here — this is only a pointer
    /// to the profile that owns it.
    pub profile: Option<i64>,
    /// Variables pinned to the watch list, per problem.
    pub watched: BTreeSet<String>,
    pub show_logs: bool,
    /// List page: hide problems that have no animation yet.
    pub viz_only: bool,
    /// List page: the progress filter, and what it is narrowed to.
    pub status_filter: StatusFilter,
    pub favourites_only: bool,
    pub playlist: Option<i64>,
    /// List page: drift the coloured lights behind the page. Off freezes them,
    /// which lets the app go back to repainting only on input.
    pub backdrop: bool,
    /// egui's zoom factor, driven by ctrl +/- and ctrl+scroll. Remembered so a
    /// comfortable size survives a restart.
    pub zoom: f32,
    /// Practice: the problem statement column.
    pub show_question: bool,
    /// Practice: the AI assist column.
    pub ai_open: bool,
    pub ollama_url: String,
    pub ollama_model: String,
    pub catalog_width: f32,
    pub inspector_width: f32,
    pub code_fraction: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: true,
            lang: "go".into(),
            tier: Tier::T150,
            selected: None,
            speed: 1.0,
            animate: true,
            profile: None,
            watched: BTreeSet::new(),
            show_logs: true,
            viz_only: false,
            status_filter: StatusFilter::All,
            favourites_only: false,
            playlist: None,
            backdrop: true,
            zoom: 1.15,
            show_question: true,
            ai_open: false,
            ollama_url: dsa_ai::DEFAULT_OLLAMA_URL.to_string(),
            ollama_model: String::new(),
            catalog_width: 290.0,
            inspector_width: 310.0,
            code_fraction: 0.44,
        }
    }
}

const KEY: &str = "dsa-visualized-settings";

impl Settings {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        storage
            .and_then(|s| s.get_string(KEY))
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn store(&self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(self) {
            storage.set_string(KEY, s);
        }
    }

    /// Watch keys are namespaced per problem so pinning `i` in Two Sum does
    /// not silently pin `i` everywhere else.
    pub fn watch_key(slug: &str, var: &str) -> String {
        format!("{slug}::{var}")
    }

    pub fn is_watched(&self, slug: &str, var: &str) -> bool {
        self.watched.contains(&Self::watch_key(slug, var))
    }

    pub fn toggle_watch(&mut self, slug: &str, var: &str) {
        let key = Self::watch_key(slug, var);
        if !self.watched.remove(&key) {
            self.watched.insert(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let s = Settings::default();
        assert!(s.dark);
        assert_eq!(s.tier, Tier::T150);
        assert!(s.animate);
        assert!(
            s.show_question,
            "the practice tab opens with the statement visible"
        );
        assert_eq!(s.ollama_url, dsa_ai::DEFAULT_OLLAMA_URL);
    }

    #[test]
    fn watches_are_scoped_to_a_problem() {
        let mut s = Settings::default();
        s.toggle_watch("two-sum", "i");
        assert!(s.is_watched("two-sum", "i"));
        assert!(!s.is_watched("binary-search", "i"));
        s.toggle_watch("two-sum", "i");
        assert!(!s.is_watched("two-sum", "i"));
    }

    #[test]
    fn settings_round_trip_through_json() {
        let s = Settings {
            profile: Some(7),
            status_filter: StatusFilter::Solved,
            playlist: Some(3),
            speed: 2.5,
            ..Default::default()
        };
        let text = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back.speed, 2.5);
        assert_eq!(back.profile, Some(7));
        assert_eq!(back.status_filter, StatusFilter::Solved);
        assert_eq!(back.playlist, Some(3));
    }

    #[test]
    fn a_blob_saved_before_profiles_existed_still_loads() {
        // The old build persisted a `solved` set here; the database owns that
        // now, and an unknown field must not cost someone their preferences.
        let old = r#"{"dark":true,"lang":"cpp","solved":["two-sum"],"tier":"50"}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.lang, "cpp");
        assert_eq!(s.tier, Tier::T50);
        assert_eq!(s.profile, None, "no profile chosen yet");
        assert_eq!(s.status_filter, StatusFilter::All);
    }

    #[test]
    fn an_older_saved_blob_still_loads() {
        // Fields added later must not invalidate a user's saved preferences.
        let old = r#"{"dark":false,"lang":"cpp"}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert!(!s.dark);
        assert_eq!(s.lang, "cpp");
        assert_eq!(s.tier, Tier::T150, "missing fields fall back to defaults");
    }
}
