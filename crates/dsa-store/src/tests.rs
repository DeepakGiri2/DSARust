use super::*;

fn store() -> Store {
    Store::in_memory().expect("in-memory database")
}

fn alice(s: &Store) -> Profile {
    s.create_profile("Alice", AVATARS[0], COLORS[0]).unwrap()
}

// ── profiles ────────────────────────────────────────────────────────────────

#[test]
fn a_fresh_database_has_no_profiles() {
    let s = store();
    assert!(s.profiles().unwrap().is_empty());
}

#[test]
fn profiles_are_created_and_listed_most_recent_first() {
    let s = store();
    let a = alice(&s);
    let b = s.create_profile("Bob", AVATARS[1], COLORS[1]).unwrap();
    s.touch_profile(a.id).unwrap();

    let names: Vec<String> = s.profiles().unwrap().into_iter().map(|p| p.name).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
    assert_eq!(s.profile(b.id).unwrap().unwrap().name, "Bob");
}

#[test]
fn there_is_no_limit_on_how_many_profiles_exist() {
    let s = store();
    for i in 0..25 {
        s.create_profile(
            &format!("player {i}"),
            AVATARS[i % AVATARS.len()],
            COLORS[i % COLORS.len()],
        )
        .unwrap();
    }
    assert_eq!(s.profiles().unwrap().len(), 25);
}

#[test]
fn a_profile_needs_a_name_and_cannot_share_one() {
    let s = store();
    alice(&s);
    assert!(matches!(
        s.create_profile("   ", AVATARS[0], COLORS[0]),
        Err(StoreError::Rejected(_))
    ));
    // Case-insensitively: "alice" and "Alice" are the same person to a reader.
    assert!(matches!(
        s.create_profile("alice", AVATARS[0], COLORS[0]),
        Err(StoreError::Rejected(_))
    ));
    assert_eq!(s.profiles().unwrap().len(), 1);
}

#[test]
fn a_profile_can_be_renamed_but_not_onto_another_name() {
    let s = store();
    let a = alice(&s);
    s.create_profile("Bob", AVATARS[1], COLORS[1]).unwrap();

    s.update_profile(a.id, "Alicia", AVATARS[2], COLORS[2])
        .unwrap();
    let back = s.profile(a.id).unwrap().unwrap();
    assert_eq!(back.name, "Alicia");
    assert_eq!(back.avatar, AVATARS[2]);

    assert!(matches!(
        s.update_profile(a.id, "Bob", AVATARS[0], COLORS[0]),
        Err(StoreError::Rejected(_))
    ));
    // Keeping its own name while changing the colour must still work.
    s.update_profile(a.id, "Alicia", AVATARS[2], COLORS[3])
        .unwrap();
}

#[test]
fn deleting_a_profile_takes_its_progress_and_playlists_with_it() {
    let s = store();
    let a = alice(&s);
    let b = s.create_profile("Bob", AVATARS[1], COLORS[1]).unwrap();
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    s.advance(b.id, "two-sum", Status::Solved).unwrap();
    let list = s.create_playlist(a.id, "revision").unwrap();
    s.add_to_playlist(list.id, "two-sum").unwrap();

    s.delete_profile(a.id).unwrap();

    assert!(s.profile(a.id).unwrap().is_none());
    assert!(s.snapshot(a.id).unwrap().is_empty());
    assert!(s.playlists(a.id).unwrap().is_empty());
    assert!(s.playlist_slugs(list.id).unwrap().is_empty());
    // Bob is untouched — this is the whole point of profiles.
    assert_eq!(s.stats(b.id).unwrap().solved, 1);
}

// ── progress ────────────────────────────────────────────────────────────────

#[test]
fn an_untouched_problem_has_no_row_and_reads_as_todo() {
    let s = store();
    let a = alice(&s);
    assert_eq!(s.entry(a.id, "two-sum").unwrap().status, Status::Todo);
    assert!(s.snapshot(a.id).unwrap().is_empty());
}

#[test]
fn progress_is_per_profile() {
    let s = store();
    let a = alice(&s);
    let b = s.create_profile("Bob", AVATARS[1], COLORS[1]).unwrap();

    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    assert_eq!(s.entry(a.id, "two-sum").unwrap().status, Status::Solved);
    assert_eq!(s.entry(b.id, "two-sum").unwrap().status, Status::Todo);
}

#[test]
fn a_failed_run_after_a_pass_does_not_un_solve_the_problem() {
    let s = store();
    let a = alice(&s);
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    // Going back to tinker records the attempt and keeps the tick.
    s.record_attempt(a.id, "two-sum").unwrap();
    let e = s.entry(a.id, "two-sum").unwrap();
    assert_eq!(e.status, Status::Solved);
    assert_eq!(e.attempts, 1);
    assert!(e.solved_at.is_some());
}

#[test]
fn attempts_accumulate_and_the_first_one_promotes_from_todo() {
    let s = store();
    let a = alice(&s);
    s.record_attempt(a.id, "two-sum").unwrap();
    assert_eq!(s.entry(a.id, "two-sum").unwrap().status, Status::Attempted);
    s.record_attempt(a.id, "two-sum").unwrap();
    s.record_attempt(a.id, "two-sum").unwrap();
    assert_eq!(s.entry(a.id, "two-sum").unwrap().attempts, 3);
}

#[test]
fn un_ticking_solved_is_possible_but_deliberate() {
    let s = store();
    let a = alice(&s);
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    s.set_status(a.id, "two-sum", Status::Todo).unwrap();
    let e = s.entry(a.id, "two-sum").unwrap();
    assert_eq!(e.status, Status::Todo);
    assert_eq!(e.solved_at, None);
}

#[test]
fn a_favourite_survives_a_status_change_and_the_other_way_round() {
    let s = store();
    let a = alice(&s);
    s.set_favourite(a.id, "two-sum", true).unwrap();
    s.advance(a.id, "two-sum", Status::Solved).unwrap();

    let e = s.entry(a.id, "two-sum").unwrap();
    assert!(e.favourite, "solving must not clear the star");
    assert_eq!(e.status, Status::Solved);

    s.set_favourite(a.id, "two-sum", false).unwrap();
    assert_eq!(s.entry(a.id, "two-sum").unwrap().status, Status::Solved);
    assert!(!s.entry(a.id, "two-sum").unwrap().favourite);
}

#[test]
fn stats_count_what_the_profile_card_shows() {
    let s = store();
    let a = alice(&s);
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    s.advance(a.id, "3sum", Status::Solved).unwrap();
    s.record_attempt(a.id, "valid-sudoku").unwrap();
    s.set_favourite(a.id, "two-sum", true).unwrap();

    let stats = s.stats(a.id).unwrap();
    assert_eq!(stats.solved, 2);
    assert_eq!(stats.attempted, 1, "solved problems are not also attempted");
    assert_eq!(stats.favourites, 1);
}

// ── playlists ───────────────────────────────────────────────────────────────

#[test]
fn playlists_hold_problems_and_report_their_size() {
    let s = store();
    let a = alice(&s);
    let list = s.create_playlist(a.id, "week 1").unwrap();
    s.add_to_playlist(list.id, "two-sum").unwrap();
    s.add_to_playlist(list.id, "3sum").unwrap();
    // Adding twice is a no-op, not an error: the button is a toggle.
    s.add_to_playlist(list.id, "3sum").unwrap();

    let lists = s.playlists(a.id).unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].len, 2);
    assert_eq!(
        s.playlist_slugs(list.id).unwrap(),
        ["3sum".to_string(), "two-sum".to_string()].into()
    );

    s.remove_from_playlist(list.id, "3sum").unwrap();
    assert_eq!(s.playlists(a.id).unwrap()[0].len, 1);
}

#[test]
fn playlists_are_per_profile_and_uniquely_named_within_one() {
    let s = store();
    let a = alice(&s);
    let b = s.create_profile("Bob", AVATARS[1], COLORS[1]).unwrap();
    s.create_playlist(a.id, "revision").unwrap();

    assert!(matches!(
        s.create_playlist(a.id, "Revision"),
        Err(StoreError::Rejected(_))
    ));
    // Bob may have his own "revision" — they are different lists.
    s.create_playlist(b.id, "revision").unwrap();
    assert_eq!(s.playlists(a.id).unwrap().len(), 1);
    assert_eq!(s.playlists(b.id).unwrap().len(), 1);
}

#[test]
fn a_deleted_playlist_takes_its_items_but_not_the_progress() {
    let s = store();
    let a = alice(&s);
    let list = s.create_playlist(a.id, "week 1").unwrap();
    s.add_to_playlist(list.id, "two-sum").unwrap();
    s.advance(a.id, "two-sum", Status::Solved).unwrap();

    s.delete_playlist(list.id).unwrap();
    assert!(s.playlist_slugs(list.id).unwrap().is_empty());
    assert_eq!(s.entry(a.id, "two-sum").unwrap().status, Status::Solved);
}

#[test]
fn the_add_to_playlist_menu_knows_which_lists_a_problem_is_in() {
    let s = store();
    let a = alice(&s);
    let one = s.create_playlist(a.id, "week 1").unwrap();
    let two = s.create_playlist(a.id, "week 2").unwrap();
    s.add_to_playlist(one.id, "two-sum").unwrap();

    let with = s.playlists_with(a.id, "two-sum").unwrap();
    assert!(with.contains(&one.id));
    assert!(!with.contains(&two.id));
}

// ── housekeeping ────────────────────────────────────────────────────────────

#[test]
fn pruning_drops_progress_for_problems_the_catalogue_lost() {
    let s = store();
    let a = alice(&s);
    let list = s.create_playlist(a.id, "week 1").unwrap();
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    s.advance(a.id, "renamed-away", Status::Solved).unwrap();
    s.add_to_playlist(list.id, "renamed-away").unwrap();

    let known = ["two-sum".to_string()].into();
    assert_eq!(s.prune(&known).unwrap(), 2);
    assert!(s.snapshot(a.id).unwrap().contains_key("two-sum"));
    assert!(!s.snapshot(a.id).unwrap().contains_key("renamed-away"));
    assert!(s.playlist_slugs(list.id).unwrap().is_empty());
}

#[test]
fn an_empty_catalogue_prunes_nothing() {
    // Content failing to load must not wipe a user's history.
    let s = store();
    let a = alice(&s);
    s.advance(a.id, "two-sum", Status::Solved).unwrap();
    assert_eq!(s.prune(&BTreeSet::new()).unwrap(), 0);
    assert_eq!(s.stats(a.id).unwrap().solved, 1);
}

#[test]
fn a_database_reopens_with_everything_still_in_it() {
    let dir = std::env::temp_dir().join(format!("dsa-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("progress.db");

    let id = {
        let s = Store::open(&path).unwrap();
        let a = s.create_profile("Alice", AVATARS[0], COLORS[0]).unwrap();
        s.advance(a.id, "two-sum", Status::Solved).unwrap();
        s.set_favourite(a.id, "3sum", true).unwrap();
        a.id
    };

    let s = Store::open(&path).unwrap();
    assert_eq!(s.profiles().unwrap().len(), 1);
    assert_eq!(s.entry(id, "two-sum").unwrap().status, Status::Solved);
    assert!(s.entry(id, "3sum").unwrap().favourite);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn statuses_order_worst_to_best_so_progress_only_moves_forward() {
    assert!(Status::Todo < Status::Attempted);
    assert!(Status::Attempted < Status::Solved);
    assert_eq!(Status::default(), Status::Todo);
}
