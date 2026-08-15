//! Runtime-loaded content for **DSA Visualized**.
//!
//! Everything a problem is made of — metadata, solution sources in every
//! language, the animated trace and its test cases — lives on disk under
//! `content/` and is read at startup. Adding a problem, or rewriting how an
//! existing one animates, never requires rebuilding the application:
//!
//! ```text
//! content/
//!   languages.toml            # which languages exist, how to run them
//!   catalog.toml              # the NeetCode lists, by category and tier
//!   lib/*.rhai                # shared helpers, prepended to every script
//!   problems/<slug>/
//!     problem.toml            # metadata, inputs, tests
//!     trace.rhai              # the animation: what to record, step by step
//!     code/<lang>.txt         # solution sources, `//@tag`-annotated
//! ```
//!
//! [`Library`] loads that tree, [`watch::ContentWatcher`] reloads it on change,
//! and [`lint`] verifies it the way a compiler would verify code.

pub mod convert;
pub mod guide;
pub mod library;
pub mod lint;
pub mod paths;
pub mod script;
pub mod watch;

pub use guide::{CheatSection, Guide, Topic, TopicKind};
pub use library::{Library, LoadError, ProblemPack};
pub use lint::{lint, Issue, Report, Severity};
pub use paths::{find_content_root, slugify, ENV_CONTENT};
pub use script::{ScriptError, ScriptHost};
pub use watch::{Change, ContentWatcher};
