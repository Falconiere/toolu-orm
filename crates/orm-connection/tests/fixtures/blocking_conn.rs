//! Runtime-free setup for the `DbConnectionBlocking` suites.
//!
//! Wired into `rusqlite_blocking_test.rs` and `rusqlite_blocking_concurrency_test.rs`
//! with `#[path]`; every item here is used by both binaries (no dead code under
//! `-D warnings`).

use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::{DbConnectionBlocking, DbError};

// Both binaries declare the row fixture as `mod rusqlite_rows` at their root.
use crate::rusqlite_rows::ScalarRow;

/// A connection built without touching tokio: `rusqlite::Connection::open_in_memory`
/// is sync and `from_connection` is neither `async` nor fallible, so nothing on
/// this path needs a runtime.
///
/// # Errors
///
/// Returns the rusqlite error if the in-memory database cannot be opened.
pub fn open_in_memory() -> Result<RusqliteConnection, rusqlite::Error> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

/// Read the single scalar a one-row, one-column query returns, over the
/// blocking path.
///
/// # Errors
///
/// Returns `DbError::Query` if the statement fails or returns no row.
pub fn scalar(conn: &RusqliteConnection, sql: &str) -> Result<i64, DbError> {
  let rows: Vec<ScalarRow> = DbConnectionBlocking::query_map(conn, sql, vec![])?;
  rows
    .first()
    .map(|row| row.value)
    .ok_or_else(|| DbError::Query(format!("no row for `{sql}`")))
}
