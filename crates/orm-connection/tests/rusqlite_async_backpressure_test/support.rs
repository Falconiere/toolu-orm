//! Real-SQLite setup for the admission-gate suites: a seeded in-memory
//! database, a runtime with a deliberately small blocking pool, and a row whose
//! decode holds the connection for a controlled time.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::{DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

pub const SELECT_LABEL: &str = "SELECT label FROM gated";
pub const INSERT_LABEL: &str = "INSERT INTO gated (label) VALUES (?1)";

/// Signalled from inside [`HoldingRow`]'s decode, which runs on the blocking
/// thread while the connection lock is held. A test that waits for it knows the
/// statement really is in flight, rather than guessing with a sleep.
pub static DECODE_STARTED: Notify = Notify::const_new();

/// How long [`HoldingRow`]'s decode holds the connection. A `static` because
/// `FromRow` decodes through an associated function with no room for state;
/// `cargo nextest` runs every test in its own process, and every test that uses
/// [`HoldingRow`] arms this first, so the value is never shared across tests.
static HOLD_MS: AtomicU64 = AtomicU64::new(0);

/// Arm the holding decode. Call before issuing the query that uses it.
pub fn hold_for(millis: u64) {
  HOLD_MS.store(millis, Ordering::SeqCst);
}

/// A row that signals, then sleeps, while `query_map` still holds the
/// connection's mutex and the live statement.
///
/// This is the fixture the admission gate is measured against: the connection,
/// its open statement and one blocking-pool thread are all occupied for the
/// whole hold, which is exactly the state a slow SQL statement produces.
pub struct HoldingRow {
  pub label: String,
}

impl FromRow for HoldingRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let label: String = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    DECODE_STARTED.notify_one();
    std::thread::sleep(Duration::from_millis(HOLD_MS.load(Ordering::SeqCst)));
    Ok(Self { label })
  }
}

/// A runtime whose blocking pool is small enough that starvation is observable:
/// one statement plus one unrelated blocking task is the entire budget.
pub fn runtime(max_blocking_threads: usize) -> Result<tokio::runtime::Runtime, std::io::Error> {
  tokio::runtime::Builder::new_multi_thread()
    .worker_threads(2)
    .max_blocking_threads(max_blocking_threads)
    .enable_all()
    .build()
}

/// An in-memory database with a `gated` table. Built synchronously, so no
/// blocking-pool thread is spent before the measurement starts.
pub fn seeded_connection() -> Result<RusqliteConnection, DbError> {
  let raw =
    rusqlite::Connection::open_in_memory().map_err(|e| DbError::Connection(e.to_string()))?;
  let conn = RusqliteConnection::from_connection(raw);
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE gated (label TEXT NOT NULL)")?;
  Ok(conn)
}

/// Insert one row over the blocking path (test setup, never measured).
pub fn insert_label(conn: &RusqliteConnection, label: &str) -> Result<u64, DbError> {
  DbConnectionBlocking::execute_sql(conn, INSERT_LABEL, vec![Value::Text(label.to_owned())])
}
