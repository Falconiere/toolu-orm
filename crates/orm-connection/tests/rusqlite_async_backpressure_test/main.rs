//! The async admission gate on `RusqliteConnection` (issue #90).
//!
//! `DbConnection`'s methods acquire one shared permit **before** calling
//! `tokio::task::spawn_blocking`, and move that owned permit into the blocking
//! closure. Waiting therefore happens on the caller's task instead of on a
//! blocking-pool thread, and a caller that drops its future cannot hand the
//! connection to a replacement while its own statement is still running.
//!
//! These suites prove, on a real in-memory SQLite database:
//!
//! - `starvation` -- the issue's reproduction: with two blocking threads and a
//!   contended connection, unrelated blocking work is still served.
//! - `cancellation` -- dropping before admission starts no SQL; dropping after
//!   admission keeps the gate closed until the statement ends.
//! - `serialization` -- concurrent async callers still serialize correctly and
//!   direct `DbConnectionBlocking` callers keep working alongside them.
//! - `errors` -- row-mapping failures are unchanged, and a panicking decode
//!   releases the permit instead of wedging the connection.
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

mod cancellation;
mod errors;
mod serialization;
mod starvation;
mod support;
