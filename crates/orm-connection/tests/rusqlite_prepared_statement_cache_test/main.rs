//! Prepared-statement reuse on `RusqliteConnection`'s `DbConnectionBlocking`
//! path (rusqlite-only lane, synchronous, no tokio runtime):
//! `execute_sql`/`query_map` now go through `Connection::prepare_cached`
//! instead of `prepare`/`execute`, inside the existing `std::sync::Mutex`
//! guard. These suites prove that reuse behaves exactly like the uncached
//! path across changed parameters, a schema change, an error followed by
//! reuse, and a row-mapping failure -- and that the guard is always released
//! (a leaked `CachedStatement` borrow would hang the next call) (issue #88).
//!
//! Schema-change safety (`reads`) rests on SQLite's own behavior, not on
//! anything this crate's adapters do: SQLite revalidates a prepared
//! statement's schema cookie on every execution and recompiles it against the
//! current schema before running, at the C-library level, whether the
//! statement handle came from `prepare` or `prepare_cached`. Caching
//! therefore adds no staleness risk beyond what the uncached path already had.
//!
//! Single-backend shape: `FromRow` exposes `from_row(&rusqlite::Row)` only when
//! rusqlite is the sole driver feature on orm-core (the rusqlite-only lane).
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

#[path = "../fixtures/blocking_conn.rs"]
pub mod blocking_conn;
#[path = "../fixtures/rusqlite_rows.rs"]
pub mod rusqlite_rows;

mod decode_failure;
mod reads;
mod support;
mod writes;
