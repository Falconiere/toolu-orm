//! Prepared-statement reuse on `RusqliteConnection`'s `DbConnectionBlocking`
//! path (rusqlite-only lane, synchronous, no tokio runtime):
//! `execute_sql`/`query_map` now go through `Connection::prepare_cached`
//! instead of `prepare`/`execute`, inside the existing `std::sync::Mutex`
//! guard. These suites prove that reuse behaves exactly like the uncached
//! path across changed parameters, a schema change, an error followed by
//! reuse, and a row-mapping failure -- and that the guard is always released
//! (a leaked `CachedStatement` borrow would hang the next call) (issue #88).
//!
//! Single-backend shape: `FromRow` exposes `from_row(&rusqlite::Row)` only when
//! rusqlite is the sole driver feature on orm-core (the rusqlite-only lane).
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::{DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

#[path = "fixtures/blocking_conn.rs"]
pub mod blocking_conn;
#[path = "fixtures/rusqlite_rows.rs"]
pub mod rusqlite_rows;

use blocking_conn::open_in_memory;
use rusqlite_rows::LabelRow;

const SELECT_BY_ID: &str = "SELECT label FROM items WHERE id = ?1";

fn create_items(
  conn: &toolu_orm_connection::rusqlite_impl::RusqliteConnection,
) -> Result<(), DbError> {
  DbConnectionBlocking::execute_batch(
    conn,
    "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
  )
}

fn insert_item(
  conn: &toolu_orm_connection::rusqlite_impl::RusqliteConnection,
  id: i64,
  label: &str,
) -> Result<u64, DbError> {
  DbConnectionBlocking::execute_sql(
    conn,
    "INSERT INTO items (id, label) VALUES (?1, ?2)",
    vec![Value::Integer(id), Value::Text(label.to_owned())],
  )
}

fn select_label(
  conn: &toolu_orm_connection::rusqlite_impl::RusqliteConnection,
  id: i64,
) -> Result<Vec<LabelRow>, DbError> {
  DbConnectionBlocking::query_map(conn, SELECT_BY_ID, vec![Value::Integer(id)])
}

/// Decodes column 0 as an integer regardless of its actual type, so selecting
/// a text column (`label`) through it forces a rusqlite type-mismatch error,
/// which `query_map` must surface as `DbError::RowMapping`. No decoded value
/// is kept -- only whether decoding itself succeeds matters.
#[derive(Debug)]
struct MismatchedRow;

impl FromRow for MismatchedRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let _: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self)
  }
}

// ── query_map (read path) ───────────────────────────────────────────────────

#[test]
fn query_map_reuses_cached_statement_across_changed_parameters()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;
  insert_item(&conn, 2, "beta")?;

  // A second call right after the first, on the same connection: a leaked
  // `CachedStatement` borrow (holding the `MutexGuard` open) would hang here.
  let first = select_label(&conn, 1)?;
  let second = select_label(&conn, 2)?;

  let first_label = first
    .first()
    .map(|row| row.label.as_str())
    .ok_or("first call should find exactly one row")?;
  assert_eq!(first_label, "alpha");
  let second_label = second
    .first()
    .map(|row| row.label.as_str())
    .ok_or("second call should find exactly one row")?;
  assert_eq!(second_label, "beta");
  Ok(())
}

#[test]
fn query_map_reuse_survives_unrelated_schema_change() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  let before = select_label(&conn, 1)?;
  let before_label = before
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row before the schema change")?;

  // Unrelated schema change: creates a new table, does not touch `items`.
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE unrelated (id INTEGER PRIMARY KEY)")?;

  let after = select_label(&conn, 1)?;
  let after_label = after
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row after the schema change")?;
  assert_eq!(
    after_label, before_label,
    "an unrelated schema change must not change the cached statement's result"
  );
  Ok(())
}

#[test]
fn query_map_reuse_reports_error_after_table_drop() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  let before = select_label(&conn, 1)?;
  assert_eq!(before.len(), 1);

  DbConnectionBlocking::execute_batch(&conn, "DROP TABLE items")?;

  match select_label(&conn, 1) {
    Err(DbError::Query(_)) => {},
    other => {
      return Err(
        format!("expected DbError::Query once the queried table is gone, got: {other:?}").into(),
      );
    },
  }
  Ok(())
}

#[test]
fn query_map_error_does_not_poison_later_reuse() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  let first = select_label(&conn, 1)?;
  let first_label = first
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row on the first call")?;

  let failing: Result<Vec<LabelRow>, DbError> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM missing_table", vec![]);
  assert!(failing.is_err(), "querying a nonexistent table must fail");

  // Same SQL text this connection already ran successfully above.
  let second = select_label(&conn, 1)?;
  let second_label = second
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row on the second call")?;
  assert_eq!(
    second_label, first_label,
    "a prior error on different SQL must not poison reuse of SQL that succeeded before"
  );
  Ok(())
}

#[test]
fn query_map_decode_failure_is_row_mapping_error() -> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  // `label` is TEXT; decoding column 0 as i64 must fail.
  let result: Result<Vec<MismatchedRow>, DbError> =
    DbConnectionBlocking::query_map(&conn, SELECT_BY_ID, vec![Value::Integer(1)]);

  match result {
    Err(DbError::RowMapping(_)) => {},
    other => return Err(format!("expected DbError::RowMapping, got: {other:?}").into()),
  }
  Ok(())
}

// ── execute_sql (write path) ────────────────────────────────────────────────

#[test]
fn execute_sql_reuses_cached_statement_across_changed_parameters()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = open_in_memory()?;
  create_items(&conn)?;

  let affected_first = insert_item(&conn, 1, "alpha")?;
  let affected_second = insert_item(&conn, 2, "beta")?;

  assert_eq!(affected_first, 1);
  assert_eq!(affected_second, 1);

  let rows: Vec<LabelRow> =
    DbConnectionBlocking::query_map(&conn, "SELECT label FROM items ORDER BY id", vec![])?;
  match rows.as_slice() {
    [alpha, beta] => {
      assert_eq!(alpha.label, "alpha");
      assert_eq!(beta.label, "beta");
    },
    other => return Err(format!("expected exactly two rows, got: {other:?}").into()),
  }
  Ok(())
}
