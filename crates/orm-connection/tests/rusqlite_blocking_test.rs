//! `DbConnectionBlocking` on `RusqliteConnection`: the runtime-free statement
//! surface, and its agreement with the async `DbConnection` path.
//!
//! Single-backend shape: `FromRow` exposes `from_row(&rusqlite::Row)` only when
//! rusqlite is the sole driver feature on orm-core (the rusqlite-only lane).
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use std::rc::Rc;

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

/// The whole statement surface with no runtime in the process: this is a plain
/// `#[test]`, so any `spawn_blocking` left on the path would fail with
/// "there is no reactor running".
#[test]
fn blocking_roundtrip_without_a_runtime() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;

  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE blocking_smoke (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
  )?;

  let affected = DbConnectionBlocking::execute_sql(
    &conn,
    "INSERT INTO blocking_smoke (id, label) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Text("no-runtime".to_owned())],
  )?;
  assert_eq!(affected, 1);

  let rows: Vec<LabelRow> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM blocking_smoke", vec![])?;
  assert_eq!(rows.len(), 1);
  assert_eq!(
    rows.first().map(|row| row.label.as_str()),
    Some("no-runtime")
  );
  assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM blocking_smoke")?, 1);
  Ok(())
}

/// A row type holding an `Rc`, which is neither `Send` nor `Sync`. The async
/// `query_map` requires `T: Send + 'static` and would reject it; the blocking
/// one decodes on the caller's own thread and needs no such bound.
struct RcRow {
  label: Rc<String>,
}

impl FromRow for RcRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let label: String = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self {
      label: Rc::new(label),
    })
  }
}

#[test]
fn query_map_accepts_a_non_send_row() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE non_send (label TEXT NOT NULL);
     INSERT INTO non_send (label) VALUES ('not-send');",
  )?;

  let rows: Vec<RcRow> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM non_send", vec![])?;

  assert_eq!(rows.len(), 1);
  assert_eq!(rows.first().map(|row| row.label.as_str()), Some("not-send"));
  Ok(())
}

/// A failing statement is an error, not a panic, and it is the same variant and
/// message the async path returns.
#[test]
fn bad_sql_returns_a_query_error() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;

  let error = DbConnectionBlocking::execute_sql(
    &conn,
    "INSERT INTO nonexistent (x) VALUES (?1)",
    vec![Value::Integer(1)],
  )
  .err()
  .ok_or("the statement should have failed")?;

  assert!(
    matches!(&error, DbError::Query(message) if message.contains("no such table")),
    "expected a DbError::Query naming the missing table, got {error:?}"
  );
  Ok(())
}

/// The async impl delegates to the blocking one, so both address the same
/// connection and run the same statement code. A second, drifted implementation
/// of the statement path would fail one of these two reads.
#[tokio::test]
async fn async_and_blocking_share_one_connection() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE shared (label TEXT NOT NULL)")?;

  DbConnectionBlocking::execute_sql(
    &conn,
    "INSERT INTO shared (label) VALUES (?1)",
    vec![Value::Text("written-blocking".to_owned())],
  )?;
  let seen_by_async: Vec<LabelRow> =
    DbConnection::query_map(&conn, "SELECT label FROM shared", vec![]).await?;
  assert_eq!(
    seen_by_async.first().map(|row| row.label.as_str()),
    Some("written-blocking")
  );

  DbConnection::execute_sql(
    &conn,
    "INSERT INTO shared (label) VALUES (?1)",
    vec![Value::Text("written-async".to_owned())],
  )
  .await?;
  let seen_by_blocking: Vec<LabelRow> = DbConnectionBlocking::query_map(
    &conn,
    "SELECT label FROM shared WHERE label = 'written-async'",
    vec![],
  )?;
  assert_eq!(
    seen_by_blocking.first().map(|row| row.label.as_str()),
    Some("written-async")
  );

  assert_eq!(scalar(&conn, "SELECT COUNT(*) FROM shared")?, 2);
  Ok(())
}
