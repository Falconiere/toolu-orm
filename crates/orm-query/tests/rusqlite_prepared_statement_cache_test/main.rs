//! Prepared-statement reuse against a real in-memory rusqlite database
//! (rusqlite-only lane, synchronous): `Executor for rusqlite::Connection`'s
//! `query_map`/`execute_sql` now go through `Connection::prepare_cached`
//! instead of `prepare`/`execute`. These suites prove that reuse behaves
//! exactly like the uncached path across changed parameters, a schema change,
//! an error followed by reuse, and a row-mapping failure (issue #88).
//!
//! Schema-change safety (`schema_changes`) rests on SQLite's own behavior,
//! not on anything this crate's adapters do: SQLite revalidates a prepared
//! statement's schema cookie on every execution and recompiles it against the
//! current schema before running, at the C-library level, whether the
//! statement handle came from `prepare` or `prepare_cached`. Caching therefore
//! adds no staleness risk beyond what the uncached path already had.

#[path = "../fixtures/rusqlite_db.rs"]
pub mod db;
#[path = "../fixtures/rusqlite_users.rs"]
pub mod users;

mod decode_failure;
mod reads;
mod writes;
