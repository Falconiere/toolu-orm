//! `DbConnectionBlocking` under contention: inside a live tokio runtime, across
//! OS threads, and after a panic unwinds out of a statement.
//!
//! Single-backend shape: `FromRow` exposes `from_row(&rusqlite::Row)` only when
//! rusqlite is the sole driver feature on orm-core (the rusqlite-only lane).
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::{DbConnection, DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

#[path = "fixtures/blocking_conn.rs"]
pub mod blocking_conn;
#[path = "fixtures/rusqlite_rows.rs"]
pub mod rusqlite_rows;

use blocking_conn::{open_in_memory, scalar};
use rusqlite_rows::LabelRow;

/// The "server that already offloads" case: a handler on a live multi-thread
/// runtime doing its database work inside its own `spawn_blocking`, with no
/// second hop underneath. This is the context `tokio::sync::Mutex::blocking_lock`
/// panics in, which is why the connection holds a `std::sync::Mutex`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_calls_work_inside_a_runtime() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE offloaded (count INTEGER NOT NULL);
     INSERT INTO offloaded (count) VALUES (7);",
  )?;

  let total = tokio::task::spawn_blocking(move || {
    DbConnectionBlocking::execute_sql(
      &conn,
      "INSERT INTO offloaded (count) VALUES (?1)",
      vec![Value::Integer(8)],
    )?;
    scalar(&conn, "SELECT SUM(count) FROM offloaded")
  })
  .await??;

  assert_eq!(total, 15);
  Ok(())
}

/// `DbConnectionBlocking: Send + Sync`, so `&RusqliteConnection` crosses OS
/// threads. The one connection sits behind a lock, so the writes serialize and
/// every one of them lands.
#[test]
fn concurrent_threads_serialize_on_the_connection() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE contended (writer TEXT NOT NULL, n INTEGER NOT NULL)",
  )?;

  let insert = |writer: &str| -> Result<(), DbError> {
    for n in 0..50 {
      DbConnectionBlocking::execute_sql(
        &conn,
        "INSERT INTO contended (writer, n) VALUES (?1, ?2)",
        vec![Value::Text(writer.to_owned()), Value::Integer(n)],
      )?;
    }
    Ok(())
  };

  let joined = std::thread::scope(|scope| {
    let first = scope.spawn(|| insert("a"));
    let second = scope.spawn(|| insert("b"));
    (first.join(), second.join())
  });
  assert!(
    matches!(&joined, (Ok(Ok(())), Ok(Ok(())))),
    "both writer threads should have finished cleanly, got {joined:?}"
  );

  assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM contended")?, 100);
  assert_eq!(
    scalar(
      &conn,
      "SELECT COUNT(DISTINCT n) FROM contended WHERE writer = 'a'"
    )?,
    50
  );
  Ok(())
}

/// Unwinds on purpose.
///
/// `panic!` is not available at this spot: `clippy::panic` is denied for the
/// whole workspace and exempts only test *functions*, while this runs inside a
/// `FromRow` impl. Removing from an empty vector is a real panic, which is all
/// the poisoning needs.
fn unwind_deliberately() -> DbCoreError {
  let mut empty: Vec<DbCoreError> = Vec::new();
  empty.remove(0)
}

/// A `FromRow` that panics mid-iteration, to unwind out of a `query_map` while
/// the connection lock is held.
struct PanicRow;

impl FromRow for PanicRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(_row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Err(unwind_deliberately())
  }
}

/// A panic inside the delegated blocking task reaches the async caller as a
/// `JoinError`, which says nothing about the statement and leaves the connection
/// in an unknown state. It is reported as `DbError::Connection`, not as a query
/// failure.
#[tokio::test]
async fn async_reports_a_panicking_task_as_a_connection_error()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE panicking (label TEXT NOT NULL);
     INSERT INTO panicking (label) VALUES ('row');",
  )?;

  let previous_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(|_info| {}));
  let outcome =
    DbConnection::query_map::<PanicRow>(&conn, "SELECT label FROM panicking", vec![]).await;
  std::panic::set_hook(previous_hook);

  let error = outcome
    .err()
    .ok_or("the panicking decode should have failed the query")?;
  assert!(
    matches!(&error, DbError::Connection(message) if message.contains("blocking task did not complete")),
    "expected the join failure to be reported as a connection error, got {error:?}"
  );
  Ok(())
}

/// After a panic unwinds through a statement the connection can be left
/// mid-transaction, so it is refused rather than handed out again: every later
/// call -- blocking or async -- reports the poisoning instead of returning
/// results.
#[tokio::test]
async fn a_poisoned_connection_reports_it() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE poisoned (label TEXT NOT NULL);
     INSERT INTO poisoned (label) VALUES ('healthy');",
  )?;

  // Control: the connection reads fine before anything panics on it.
  let before: Vec<LabelRow> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM poisoned", vec![])?;
  assert_eq!(
    before.first().map(|row| row.label.as_str()),
    Some("healthy")
  );

  let previous_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(|_info| {}));
  let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    DbConnectionBlocking::query_map::<PanicRow>(&conn, "SELECT label FROM poisoned", vec![])
  }));
  std::panic::set_hook(previous_hook);
  assert!(unwound.is_err(), "the decoding panic should have unwound");

  let blocking_error = DbConnectionBlocking::execute_batch(&conn, "SELECT 1")
    .err()
    .ok_or("the blocking path should refuse a poisoned connection")?;
  assert!(
    matches!(&blocking_error, DbError::Connection(message) if message.contains("poisoned")),
    "expected a poisoning report, got {blocking_error:?}"
  );

  let async_error = DbConnection::execute_batch(&conn, "SELECT 1")
    .await
    .err()
    .ok_or("the async path should refuse a poisoned connection too")?;
  assert!(
    matches!(&async_error, DbError::Connection(message) if message.contains("poisoned")),
    "expected the same poisoning report on the async path, got {async_error:?}"
  );
  Ok(())
}
