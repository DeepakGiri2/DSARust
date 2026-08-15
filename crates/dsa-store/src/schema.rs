//! The database's shape, in one place.
//!
//! Bump [`SCHEMA_VERSION`] and add to [`CREATE`] when the shape changes. Every
//! statement is `IF NOT EXISTS`, so re-running it over an existing database is
//! how a migration adds a table without touching what is already there.

pub const SCHEMA_VERSION: i64 = 1;

pub const CREATE: &str = r#"
CREATE TABLE IF NOT EXISTS profile (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,
    avatar       TEXT    NOT NULL,
    color        TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL
);

-- Two profiles with the same name are indistinguishable in the picker, which
-- shows the name and nothing else.
CREATE UNIQUE INDEX IF NOT EXISTS profile_name ON profile (name COLLATE NOCASE);

-- A row exists only once a problem has been touched: "to do" is the absence of
-- one, so an untouched 287-problem catalogue costs nothing.
CREATE TABLE IF NOT EXISTS progress (
    profile_id INTEGER NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
    slug       TEXT    NOT NULL,
    status     TEXT    NOT NULL DEFAULT 'todo',
    favourite  INTEGER NOT NULL DEFAULT 0,
    attempts   INTEGER NOT NULL DEFAULT 0,
    solved_at  INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (profile_id, slug)
);

CREATE INDEX IF NOT EXISTS progress_profile ON progress (profile_id, status);

CREATE TABLE IF NOT EXISTS playlist (
    id         INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS playlist_name
    ON playlist (profile_id, name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS playlist_item (
    playlist_id INTEGER NOT NULL REFERENCES playlist(id) ON DELETE CASCADE,
    slug        TEXT    NOT NULL,
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, slug)
);
"#;
