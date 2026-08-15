//! The current profile's progress, held in memory and written through to the
//! database.
//!
//! The list paints hundreds of rows every frame and filters them as it goes, so
//! it cannot ask SQLite anything per row. Instead the whole of one profile is
//! read once — into a map the size of "problems this person has touched", which
//! is small — and every mutation updates both the database and the copy. The
//! screen therefore never blocks on a query, and the database is never behind.
//!
//! Failures are recorded rather than propagated. A read-only disk should cost
//! you your history, not the ability to use the app, so the store falls back to
//! an in-memory database and the reason is shown in the status strip.

use dsa_store::{Entry, Playlist, Profile, Stats, Status, Store};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The list screen's progress filter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusFilter {
    #[default]
    All,
    Todo,
    Attempted,
    Solved,
}

impl StatusFilter {
    pub const ALL: [StatusFilter; 4] = [
        StatusFilter::All,
        StatusFilter::Todo,
        StatusFilter::Attempted,
        StatusFilter::Solved,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            StatusFilter::All => "all",
            StatusFilter::Todo => "to do",
            StatusFilter::Attempted => "attempted",
            StatusFilter::Solved => "solved",
        }
    }

    pub fn accepts(&self, status: Status) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Todo => status == Status::Todo,
            StatusFilter::Attempted => status == Status::Attempted,
            StatusFilter::Solved => status == Status::Solved,
        }
    }
}

pub struct Progress {
    store: Store,
    profile: Option<Profile>,
    entries: BTreeMap<String, Entry>,
    playlists: Vec<Playlist>,
    /// Membership of every playlist, so the filter and the "add to" menu can
    /// both answer without a query.
    members: BTreeMap<i64, BTreeSet<String>>,
    /// Why the database is not the one on disk, if it is not.
    pub warning: Option<String>,
}

impl Progress {
    /// Open the database at `path`, degrading to an in-memory one on failure.
    pub fn open(path: &Path) -> Self {
        let (store, warning) = match Store::open(path) {
            Ok(s) => (s, None),
            Err(e) => {
                log::warn!("progress database unavailable: {e}");
                let note = format!(
                    "progress is not being saved — could not open {}: {e}",
                    path.display()
                );
                match Store::in_memory() {
                    Ok(s) => (s, Some(note)),
                    // An in-memory database touches no disk, so this means
                    // SQLite itself is unusable and there is nothing left to
                    // degrade to. Better a clear message than a window whose
                    // every button silently does nothing.
                    Err(e2) => panic!("no usable database: {e} / {e2}"),
                }
            }
        };
        Self {
            store,
            profile: None,
            entries: BTreeMap::new(),
            playlists: Vec::new(),
            members: BTreeMap::new(),
            warning,
        }
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self {
            store: Store::in_memory().expect("in-memory database"),
            profile: None,
            entries: BTreeMap::new(),
            playlists: Vec::new(),
            members: BTreeMap::new(),
            warning: None,
        }
    }

    /// Record a failed write rather than unwinding through the frame.
    fn ok<T>(&mut self, result: dsa_store::Result<T>) -> Option<T> {
        match result {
            Ok(v) => Some(v),
            Err(e) => {
                log::warn!("progress write failed: {e}");
                self.warning = Some(e.to_string());
                None
            }
        }
    }

    // ── profiles ────────────────────────────────────────────────────────────

    pub fn profiles(&self) -> Vec<Profile> {
        self.store.profiles().unwrap_or_default()
    }

    pub fn current(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    pub fn profile_id(&self) -> Option<i64> {
        self.profile.as_ref().map(|p| p.id)
    }

    /// Make `id` the active profile and load everything it knows.
    pub fn enter(&mut self, id: i64) -> bool {
        match self.store.profile(id) {
            Ok(Some(p)) => {
                let _ = self.store.touch_profile(id);
                self.profile = Some(p);
                self.reload();
                true
            }
            _ => false,
        }
    }

    pub fn leave(&mut self) {
        self.profile = None;
        self.entries.clear();
        self.playlists.clear();
        self.members.clear();
    }

    pub fn reload(&mut self) {
        let Some(id) = self.profile_id() else { return };
        self.entries = self.store.snapshot(id).unwrap_or_default();
        self.playlists = self.store.playlists(id).unwrap_or_default();
        self.members = self
            .playlists
            .iter()
            .map(|p| (p.id, self.store.playlist_slugs(p.id).unwrap_or_default()))
            .collect();
    }

    pub fn create_profile(&mut self, name: &str, avatar: &str, color: &str) -> Option<Profile> {
        let r = self.store.create_profile(name, avatar, color);
        self.ok(r)
    }

    pub fn update_profile(&mut self, id: i64, name: &str, avatar: &str, color: &str) -> bool {
        let r = self.store.update_profile(id, name, avatar, color);
        if self.ok(r).is_none() {
            return false;
        }
        if self.profile_id() == Some(id) {
            self.profile = self.store.profile(id).ok().flatten();
        }
        true
    }

    pub fn delete_profile(&mut self, id: i64) {
        let r = self.store.delete_profile(id);
        self.ok(r);
        if self.profile_id() == Some(id) {
            self.leave();
        }
    }

    /// Another profile's totals — one query, for the picker's cards.
    pub fn stats_for(&self, id: i64) -> Stats {
        if self.profile_id() == Some(id) {
            return self.stats();
        }
        self.store.stats(id).unwrap_or_default()
    }

    /// The current profile's totals, counted from the cache.
    ///
    /// The catalogue header shows these, so this runs every frame — and the
    /// backdrop animation means "every frame" is thirty times a second. Asking
    /// SQLite that often for a number that only changes on a click would undo
    /// the point of holding a snapshot at all.
    pub fn stats(&self) -> Stats {
        if self.profile.is_none() {
            return Stats::default();
        }
        Stats {
            solved: self
                .entries
                .values()
                .filter(|e| e.status == Status::Solved)
                .count(),
            attempted: self
                .entries
                .values()
                .filter(|e| e.status == Status::Attempted)
                .count(),
            favourites: self.entries.values().filter(|e| e.favourite).count(),
        }
    }

    // ── progress ────────────────────────────────────────────────────────────

    pub fn entry(&self, slug: &str) -> Entry {
        self.entries.get(slug).cloned().unwrap_or_default()
    }

    pub fn status(&self, slug: &str) -> Status {
        self.entries.get(slug).map(|e| e.status).unwrap_or_default()
    }

    pub fn is_favourite(&self, slug: &str) -> bool {
        self.entries.get(slug).is_some_and(|e| e.favourite)
    }

    /// Pressing Run or the test button: always an attempt, never a downgrade.
    pub fn record_attempt(&mut self, slug: &str) {
        let Some(id) = self.profile_id() else { return };
        let r = self.store.record_attempt(id, slug);
        if self.ok(r).is_some() {
            let e = self.entries.entry(slug.to_string()).or_default();
            e.attempts += 1;
            e.status = e.status.max(Status::Attempted);
        }
    }

    /// Every test case passed. Promotes to solved and never back.
    pub fn mark_solved(&mut self, slug: &str) {
        let Some(id) = self.profile_id() else { return };
        let r = self.store.advance(id, slug, Status::Solved);
        if self.ok(r).is_some() {
            let e = self.entries.entry(slug.to_string()).or_default();
            e.status = Status::Solved;
            e.solved_at.get_or_insert_with(dsa_store::now);
        }
    }

    /// The manual tick — the only way back to "to do".
    pub fn toggle_solved(&mut self, slug: &str) {
        let Some(id) = self.profile_id() else { return };
        let next = if self.status(slug) == Status::Solved {
            Status::Todo
        } else {
            Status::Solved
        };
        let r = self.store.set_status(id, slug, next);
        if self.ok(r).is_some() {
            let e = self.entries.entry(slug.to_string()).or_default();
            e.status = next;
            e.solved_at = (next == Status::Solved).then(dsa_store::now);
        }
    }

    pub fn toggle_favourite(&mut self, slug: &str) {
        let Some(id) = self.profile_id() else { return };
        let next = !self.is_favourite(slug);
        let r = self.store.set_favourite(id, slug, next);
        if self.ok(r).is_some() {
            self.entries.entry(slug.to_string()).or_default().favourite = next;
        }
    }

    // ── playlists ───────────────────────────────────────────────────────────

    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists
    }

    pub fn playlist(&self, id: i64) -> Option<&Playlist> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn playlist_contains(&self, id: i64, slug: &str) -> bool {
        self.members.get(&id).is_some_and(|s| s.contains(slug))
    }

    pub fn create_playlist(&mut self, name: &str) -> Option<i64> {
        let id = self.profile_id()?;
        let r = self.store.create_playlist(id, name);
        let list = self.ok(r)?;
        self.reload();
        Some(list.id)
    }

    pub fn delete_playlist(&mut self, id: i64) {
        let r = self.store.delete_playlist(id);
        self.ok(r);
        self.reload();
    }

    pub fn toggle_in_playlist(&mut self, id: i64, slug: &str) {
        let r = if self.playlist_contains(id, slug) {
            self.store.remove_from_playlist(id, slug)
        } else {
            self.store.add_to_playlist(id, slug)
        };
        if self.ok(r).is_some() {
            self.reload();
        }
    }

    // ── filtering ───────────────────────────────────────────────────────────

    /// Does this problem pass the progress-side filters?
    ///
    /// Split out from the list's own tier/search/interactive test so it can be
    /// tested without a window, and so both the row loop and the "N shown"
    /// count are guaranteed to agree.
    pub fn accepts(
        &self,
        slug: &str,
        status: StatusFilter,
        favourites_only: bool,
        playlist: Option<i64>,
    ) -> bool {
        if !status.accepts(self.status(slug)) {
            return false;
        }
        if favourites_only && !self.is_favourite(slug) {
            return false;
        }
        match playlist {
            // A playlist that has been deleted filters nothing, rather than
            // hiding the whole catalogue behind a filter you cannot see.
            Some(id) if self.playlist(id).is_some() => self.playlist_contains(id, slug),
            _ => true,
        }
    }

    /// Drop rows for problems the catalogue no longer has.
    pub fn prune(&mut self, known: &BTreeSet<String>) {
        let r = self.store.prune(known);
        if let Some(n) = self.ok(r) {
            if n > 0 {
                log::info!("pruned {n} progress row(s) for problems no longer in the catalogue");
                self.reload();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_alice() -> Progress {
        let mut p = Progress::in_memory();
        let alice = p.create_profile("Alice", "🎓", "#7c6cff").unwrap();
        assert!(p.enter(alice.id));
        p
    }

    #[test]
    fn nothing_is_recorded_until_a_profile_is_chosen() {
        let mut p = Progress::in_memory();
        p.record_attempt("two-sum");
        p.toggle_favourite("two-sum");
        p.mark_solved("two-sum");
        assert_eq!(p.status("two-sum"), Status::Todo);
        assert!(!p.is_favourite("two-sum"));
        assert!(p.warning.is_none(), "a missing profile is not an error");
    }

    #[test]
    fn the_cache_and_the_database_agree_after_every_mutation() {
        let mut p = with_alice();
        p.record_attempt("two-sum");
        p.toggle_favourite("3sum");
        p.mark_solved("valid-sudoku");

        let cached: Vec<(String, Status, bool)> = ["two-sum", "3sum", "valid-sudoku"]
            .iter()
            .map(|s| (s.to_string(), p.status(s), p.is_favourite(s)))
            .collect();
        p.reload(); // straight from SQLite
        for (slug, status, fav) in cached {
            assert_eq!(p.status(&slug), status, "{slug} status drifted");
            assert_eq!(p.is_favourite(&slug), fav, "{slug} star drifted");
        }
    }

    #[test]
    fn running_a_solved_problem_again_keeps_the_tick() {
        let mut p = with_alice();
        p.mark_solved("two-sum");
        p.record_attempt("two-sum");
        assert_eq!(p.status("two-sum"), Status::Solved);
        assert_eq!(p.entry("two-sum").attempts, 1);
    }

    #[test]
    fn the_manual_tick_goes_both_ways() {
        let mut p = with_alice();
        p.toggle_solved("two-sum");
        assert_eq!(p.status("two-sum"), Status::Solved);
        p.toggle_solved("two-sum");
        assert_eq!(p.status("two-sum"), Status::Todo);
        assert_eq!(p.entry("two-sum").solved_at, None);
    }

    #[test]
    fn the_cached_totals_match_what_the_database_would_say() {
        // The header counts from the cache to stay off the query path; if the
        // two ever disagree, the number on screen is the wrong one.
        let mut p = with_alice();
        p.mark_solved("two-sum");
        p.mark_solved("3sum");
        p.record_attempt("valid-sudoku");
        p.toggle_favourite("two-sum");

        let cached = p.stats();
        assert_eq!(cached.solved, 2);
        assert_eq!(cached.attempted, 1);
        assert_eq!(cached.favourites, 1);
        assert_eq!(cached, p.store.stats(p.profile_id().unwrap()).unwrap());
    }

    #[test]
    fn switching_profile_switches_the_whole_view() {
        let mut p = with_alice();
        p.mark_solved("two-sum");
        let bob = p.create_profile("Bob", "🎯", "#22d3ee").unwrap();

        p.enter(bob.id);
        assert_eq!(p.status("two-sum"), Status::Todo);
        assert_eq!(p.stats().solved, 0);

        let alice = p
            .profiles()
            .into_iter()
            .find(|x| x.name == "Alice")
            .unwrap();
        p.enter(alice.id);
        assert_eq!(p.status("two-sum"), Status::Solved);
        assert_eq!(p.stats().solved, 1);
    }

    #[test]
    fn the_status_filter_partitions_the_catalogue() {
        let mut p = with_alice();
        p.mark_solved("solved-one");
        p.record_attempt("tried-one");

        let cases = [
            (
                StatusFilter::All,
                ["solved-one", "tried-one", "fresh-one"].len(),
            ),
            (StatusFilter::Solved, 1),
            (StatusFilter::Attempted, 1),
            (StatusFilter::Todo, 1),
        ];
        for (filter, want) in cases {
            let n = ["solved-one", "tried-one", "fresh-one"]
                .iter()
                .filter(|s| p.accepts(s, filter, false, None))
                .count();
            assert_eq!(n, want, "{filter:?} matched the wrong rows");
        }
    }

    #[test]
    fn favourites_and_status_filter_together() {
        let mut p = with_alice();
        p.mark_solved("two-sum");
        p.toggle_favourite("two-sum");
        p.toggle_favourite("3sum");

        assert!(p.accepts("two-sum", StatusFilter::Solved, true, None));
        // Starred but not solved.
        assert!(!p.accepts("3sum", StatusFilter::Solved, true, None));
        // Solved but the star is what is being asked for.
        assert!(p.accepts("3sum", StatusFilter::All, true, None));
        assert!(!p.accepts("valid-sudoku", StatusFilter::All, true, None));
    }

    #[test]
    fn a_playlist_filter_shows_only_its_members() {
        let mut p = with_alice();
        let week = p.create_playlist("week 1").unwrap();
        p.toggle_in_playlist(week, "two-sum");

        assert!(p.accepts("two-sum", StatusFilter::All, false, Some(week)));
        assert!(!p.accepts("3sum", StatusFilter::All, false, Some(week)));

        p.toggle_in_playlist(week, "two-sum"); // toggles back off
        assert!(!p.accepts("two-sum", StatusFilter::All, false, Some(week)));
        assert_eq!(p.playlist(week).unwrap().len, 0);
    }

    #[test]
    fn a_deleted_playlist_stops_filtering_instead_of_hiding_everything() {
        let mut p = with_alice();
        let week = p.create_playlist("week 1").unwrap();
        p.toggle_in_playlist(week, "two-sum");
        p.delete_playlist(week);

        assert!(p.accepts("two-sum", StatusFilter::All, false, Some(week)));
        assert!(p.accepts("3sum", StatusFilter::All, false, Some(week)));
    }

    #[test]
    fn playlists_belong_to_the_profile_that_made_them() {
        let mut p = with_alice();
        p.create_playlist("week 1").unwrap();
        let bob = p.create_profile("Bob", "🎯", "#22d3ee").unwrap();
        p.enter(bob.id);
        assert!(p.playlists().is_empty());
    }

    #[test]
    fn a_rejected_profile_name_surfaces_rather_than_silently_failing() {
        let mut p = Progress::in_memory();
        p.create_profile("Alice", "🎓", "#7c6cff").unwrap();
        assert!(p.create_profile("alice", "🎓", "#7c6cff").is_none());
        assert!(p.warning.as_deref().is_some_and(|w| w.contains("already")));
    }
}
