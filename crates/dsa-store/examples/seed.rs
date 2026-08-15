//! Fill the real database with a few profiles, for looking at the UI.
//! `cargo run -p dsa-store --example seed -- <path>`
use dsa_store::{Status, Store, AVATARS, COLORS};

fn main() {
    let path = std::env::args().nth(1).expect("usage: seed <db path>");
    let s = Store::open(&path).unwrap();
    let alex = s.create_profile("Alex", AVATARS[0], COLORS[0]).unwrap();
    let priya = s.create_profile("Priya", AVATARS[3], COLORS[1]).unwrap();
    s.create_profile("interview prep", AVATARS[2], COLORS[3])
        .unwrap();

    for slug in [
        "two-sum",
        "valid-anagram",
        "contains-duplicate",
        "group-anagrams",
        "valid-palindrome",
    ] {
        s.advance(alex.id, slug, Status::Solved).unwrap();
    }
    for slug in ["3sum", "top-k-frequent-elements"] {
        s.record_attempt(alex.id, slug).unwrap();
    }
    for slug in ["two-sum", "trapping-rain-water", "3sum"] {
        s.set_favourite(alex.id, slug, true).unwrap();
    }
    let hard = s.create_playlist(alex.id, "hard ones").unwrap();
    for slug in ["trapping-rain-water", "container-with-most-water", "3sum"] {
        s.add_to_playlist(hard.id, slug).unwrap();
    }
    s.create_playlist(alex.id, "week 1").unwrap();
    s.advance(priya.id, "two-sum", Status::Solved).unwrap();
    println!("seeded {path}");
}
