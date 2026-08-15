//! Who is practising, and how far they have got.
//!
//! Everything in here is *per profile*, and profiles are the Netflix kind:
//! a name, a face and no password. Several people share one machine, or one
//! person keeps a "first pass" and a "revision" run side by side, and nothing
//! about that needs an account.
//!
//! The content library is read-only and lives on disk as files; this is the
//! opposite — small, write-heavy, queried by predicates ("solved, favourite,
//! in this playlist") — so it is a database rather than another folder of TOML.
//!
//! ```text
//! profile ──┬── progress       (one row per problem the profile has touched)
//!           └── playlist ───── playlist_item
//! ```
//!
//! A `progress` row is only written once a problem has been *touched*, so an
//! untouched catalogue costs nothing and [`Status::Todo`] is simply the absence
//! of a row.

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

mod schema;

pub use schema::SCHEMA_VERSION;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("{0}")]
    Rejected(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Seconds since the epoch. Passed in explicitly by the callers that care about
/// ordering so tests do not have to sleep.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// Rows
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    /// A single glyph, chosen from [`AVATARS`].
    pub avatar: String,
    /// `#rrggbb`, chosen from [`COLORS`].
    pub color: String,
    pub created_at: i64,
    pub last_seen_at: i64,
}

/// Where a problem stands for one profile.
///
/// Ordered worst-to-best so `max` is "the furthest this has got", which is what
/// re-running a solved problem and failing should *not* undo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    #[default]
    Todo,
    Attempted,
    Solved,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Attempted => "attempted",
            Status::Solved => "solved",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Status::Todo => "to do",
            Status::Attempted => "attempted",
            Status::Solved => "solved",
        }
    }
    fn parse(s: &str) -> Status {
        match s {
            "solved" => Status::Solved,
            "attempted" => Status::Attempted,
            _ => Status::Todo,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub status: Status,
    pub favourite: bool,
    /// How many times Run or the test button has been pressed on it.
    pub attempts: i64,
    pub solved_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub solved: usize,
    pub attempted: usize,
    pub favourites: usize,
}

/// The faces a profile can wear. Every one of these is checked to exist in the
/// fonts the app ships — an avatar that renders as an empty box is worse than
/// having fewer to choose from.
pub const AVATARS: [&str; 12] = [
    "🎓", "🎯", "⚡", "🔮", "🎧", "🕹", "🔑", "🗝", "📐", "🔍", "⭐", "✨",
];

/// Card colours, taken from the app's own palette.
pub const COLORS: [&str; 6] = [
    "#7c6cff", "#22d3ee", "#34d399", "#fbbf24", "#f87171", "#f472b6",
];

// ─────────────────────────────────────────────────────────────────────────────
// The store
// ─────────────────────────────────────────────────────────────────────────────

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (creating if needed) the database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::from_connection(Connection::open(path)?)
    }

    /// A database that exists only for this process — the fallback when the
    /// real one cannot be opened, and what the tests use.
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        // Foreign keys are off by default in SQLite, which would leave a
        // deleted profile's progress behind forever.
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        if version < SCHEMA_VERSION {
            self.conn.execute_batch(schema::CREATE)?;
            self.conn
                .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
        }
        Ok(())
    }

    // ── profiles ────────────────────────────────────────────────────────────

    /// Every profile, most recently used first — the order a picker wants.
    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, avatar, color, created_at, last_seen_at
             FROM profile ORDER BY last_seen_at DESC, id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Profile {
                id: r.get(0)?,
                name: r.get(1)?,
                avatar: r.get(2)?,
                color: r.get(3)?,
                created_at: r.get(4)?,
                last_seen_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn profile(&self, id: i64) -> Result<Option<Profile>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, avatar, color, created_at, last_seen_at
                 FROM profile WHERE id = ?1",
                params![id],
                |r| {
                    Ok(Profile {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        avatar: r.get(2)?,
                        color: r.get(3)?,
                        created_at: r.get(4)?,
                        last_seen_at: r.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    /// Names are trimmed and must be unique and non-empty — the picker shows
    /// nothing but the name, so two profiles called "Sam" cannot be told apart.
    pub fn create_profile(&self, name: &str, avatar: &str, color: &str) -> Result<Profile> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Rejected("a profile needs a name".into()));
        }
        if self.name_taken(name, None)? {
            return Err(StoreError::Rejected(format!("“{name}” already exists")));
        }
        let at = now();
        self.conn.execute(
            "INSERT INTO profile (name, avatar, color, created_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![name, avatar, color, at],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Profile {
            id,
            name: name.to_string(),
            avatar: avatar.to_string(),
            color: color.to_string(),
            created_at: at,
            last_seen_at: at,
        })
    }

    pub fn update_profile(&self, id: i64, name: &str, avatar: &str, color: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Rejected("a profile needs a name".into()));
        }
        if self.name_taken(name, Some(id))? {
            return Err(StoreError::Rejected(format!("“{name}” already exists")));
        }
        self.conn.execute(
            "UPDATE profile SET name = ?2, avatar = ?3, color = ?4 WHERE id = ?1",
            params![id, name, avatar, color],
        )?;
        Ok(())
    }

    /// Deletes the profile and, by way of the foreign keys, everything it knew.
    pub fn delete_profile(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM profile WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn touch_profile(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE profile SET last_seen_at = ?2 WHERE id = ?1",
            params![id, now()],
        )?;
        Ok(())
    }

    fn name_taken(&self, name: &str, except: Option<i64>) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM profile WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, except],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    // ── progress ────────────────────────────────────────────────────────────

    /// Everything one profile has recorded, in a single query.
    ///
    /// The list screen filters and paints hundreds of rows every frame; asking
    /// the database per row would be a query storm for data that changes only
    /// when the user clicks something.
    pub fn snapshot(&self, profile: i64) -> Result<BTreeMap<String, Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT slug, status, favourite, attempts, solved_at
             FROM progress WHERE profile_id = ?1",
        )?;
        let rows = stmt.query_map(params![profile], |r| {
            let slug: String = r.get(0)?;
            let status: String = r.get(1)?;
            Ok((
                slug,
                Entry {
                    status: Status::parse(&status),
                    favourite: r.get::<_, i64>(2)? != 0,
                    attempts: r.get(3)?,
                    solved_at: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn entry(&self, profile: i64, slug: &str) -> Result<Entry> {
        Ok(self.snapshot(profile)?.remove(slug).unwrap_or_default())
    }

    /// Record a status, never downgrading one.
    ///
    /// Pressing Run on a problem you already solved records another attempt; it
    /// does not un-solve it. Un-solving is a deliberate act — [`Self::clear`].
    pub fn advance(&self, profile: i64, slug: &str, status: Status) -> Result<()> {
        let at = now();
        let solved_at = (status == Status::Solved).then_some(at);
        self.conn.execute(
            "INSERT INTO progress (profile_id, slug, status, solved_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(profile_id, slug) DO UPDATE SET
                 status     = CASE WHEN excluded.status = 'solved'
                                    OR (excluded.status = 'attempted' AND status = 'todo')
                                   THEN excluded.status ELSE status END,
                 solved_at  = COALESCE(solved_at, excluded.solved_at),
                 updated_at = excluded.updated_at",
            params![profile, slug, status.as_str(), solved_at, at],
        )?;
        Ok(())
    }

    /// Force a status, including back down to [`Status::Todo`] — what the
    /// "solved" tick does when it is un-ticked.
    pub fn set_status(&self, profile: i64, slug: &str, status: Status) -> Result<()> {
        let at = now();
        let solved_at = (status == Status::Solved).then_some(at);
        self.conn.execute(
            "INSERT INTO progress (profile_id, slug, status, solved_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(profile_id, slug) DO UPDATE SET
                 status = excluded.status,
                 solved_at = excluded.solved_at,
                 updated_at = excluded.updated_at",
            params![profile, slug, status.as_str(), solved_at, at],
        )?;
        Ok(())
    }

    /// One more Run or test press, and at least "attempted".
    pub fn record_attempt(&self, profile: i64, slug: &str) -> Result<()> {
        self.advance(profile, slug, Status::Attempted)?;
        self.conn.execute(
            "UPDATE progress SET attempts = attempts + 1
             WHERE profile_id = ?1 AND slug = ?2",
            params![profile, slug],
        )?;
        Ok(())
    }

    pub fn set_favourite(&self, profile: i64, slug: &str, favourite: bool) -> Result<()> {
        let at = now();
        self.conn.execute(
            "INSERT INTO progress (profile_id, slug, status, favourite, updated_at)
             VALUES (?1, ?2, 'todo', ?3, ?4)
             ON CONFLICT(profile_id, slug) DO UPDATE SET
                 favourite = excluded.favourite,
                 updated_at = excluded.updated_at",
            params![profile, slug, favourite as i64, at],
        )?;
        Ok(())
    }

    pub fn stats(&self, profile: i64) -> Result<Stats> {
        let snapshot = self.snapshot(profile)?;
        Ok(Stats {
            solved: snapshot
                .values()
                .filter(|e| e.status == Status::Solved)
                .count(),
            attempted: snapshot
                .values()
                .filter(|e| e.status == Status::Attempted)
                .count(),
            favourites: snapshot.values().filter(|e| e.favourite).count(),
        })
    }

    // ── playlists ───────────────────────────────────────────────────────────

    pub fn playlists(&self, profile: i64) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, COUNT(i.slug)
             FROM playlist p LEFT JOIN playlist_item i ON i.playlist_id = p.id
             WHERE p.profile_id = ?1
             GROUP BY p.id, p.name
             ORDER BY p.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map(params![profile], |r| {
            Ok(Playlist {
                id: r.get(0)?,
                name: r.get(1)?,
                len: r.get::<_, i64>(2)? as usize,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn create_playlist(&self, profile: i64, name: &str) -> Result<Playlist> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StoreError::Rejected("a playlist needs a name".into()));
        }
        let taken: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM playlist WHERE profile_id = ?1 AND name = ?2 COLLATE NOCASE",
            params![profile, name],
            |r| r.get(0),
        )?;
        if taken > 0 {
            return Err(StoreError::Rejected(format!("“{name}” already exists")));
        }
        self.conn.execute(
            "INSERT INTO playlist (profile_id, name, created_at) VALUES (?1, ?2, ?3)",
            params![profile, name, now()],
        )?;
        Ok(Playlist {
            id: self.conn.last_insert_rowid(),
            name: name.to_string(),
            len: 0,
        })
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM playlist WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn playlist_slugs(&self, id: i64) -> Result<BTreeSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT slug FROM playlist_item WHERE playlist_id = ?1")?;
        let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Adding twice is not an error — the button is a toggle, and a second
    /// click from a stale view should not blow up.
    pub fn add_to_playlist(&self, id: i64, slug: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO playlist_item (playlist_id, slug, added_at)
             VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            params![id, slug, now()],
        )?;
        Ok(())
    }

    pub fn remove_from_playlist(&self, id: i64, slug: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM playlist_item WHERE playlist_id = ?1 AND slug = ?2",
            params![id, slug],
        )?;
        Ok(())
    }

    /// Every playlist a slug belongs to, for the "add to playlist" menu.
    pub fn playlists_with(&self, profile: i64, slug: &str) -> Result<BTreeSet<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT i.playlist_id FROM playlist_item i
             JOIN playlist p ON p.id = i.playlist_id
             WHERE p.profile_id = ?1 AND i.slug = ?2",
        )?;
        let rows = stmt.query_map(params![profile, slug], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Drop rows for problems the catalogue no longer has.
    ///
    /// Content is editable at runtime, so a renamed slug would otherwise leave
    /// progress stranded against a problem that no longer exists.
    pub fn prune(&self, known: &BTreeSet<String>) -> Result<usize> {
        if known.is_empty() {
            return Ok(0); // an empty catalogue means content failed to load
        }
        let mut removed = 0;
        for table in ["progress", "playlist_item"] {
            let mut stmt = self
                .conn
                .prepare(&format!("SELECT DISTINCT slug FROM {table}"))?;
            let slugs: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<_, _>>()?;
            for slug in slugs.iter().filter(|s| !known.contains(*s)) {
                removed += self.conn.execute(
                    &format!("DELETE FROM {table} WHERE slug = ?1"),
                    params![slug],
                )?;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests;
