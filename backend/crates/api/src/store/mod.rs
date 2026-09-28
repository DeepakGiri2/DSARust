//! SQL, grouped by what it is about. Handlers call these functions and never
//! build queries themselves, so every statement that touches a table can be
//! found in one file.
//!
//! Queries are checked at run time (`query_as` + `FromRow`) rather than at
//! compile time, which keeps builds independent of a live database; the
//! integration tests in `tests/` execute every one of them against Postgres.

pub mod accounts;
pub mod metering;
pub mod practice;
