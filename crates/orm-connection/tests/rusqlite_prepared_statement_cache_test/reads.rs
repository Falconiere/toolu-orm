//! `query_map` (read path): reuse across changed parameters, schema-change
//! safety, and that one call's error never poisons a later reuse.

use toolu_orm_connection::{DbConnectionBlocking, DbError};

use crate::blocking_conn::open_in_memory;
use crate::rusqlite_rows::LabelRow;
use crate::support::{create_items, insert_item, select_label};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn query_map_reuses_cached_statement_across_changed_parameters() -> TestResult {
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
fn query_map_reuse_survives_unrelated_schema_change() -> TestResult {
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
fn query_map_reuse_reports_error_after_table_drop() -> TestResult {
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
fn query_map_reuse_survives_added_column_on_the_queried_table() -> TestResult {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  let before = select_label(&conn, 1)?;
  let before_label = before
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row before the schema change")?;

  // A schema change to the queried table itself, but one that leaves the
  // `label` column SELECT_BY_ID references untouched. See the crate doc for
  // why this is safe: SQLite, not the adapter, revalidates and recompiles.
  DbConnectionBlocking::execute_batch(&conn, "ALTER TABLE items ADD COLUMN extra TEXT")?;

  let after = select_label(&conn, 1)?;
  let after_label = after
    .first()
    .map(|row| row.label.clone())
    .ok_or("expected one row after the schema change")?;
  assert_eq!(
    after_label, before_label,
    "ALTER TABLE ADD COLUMN on the queried table must not corrupt a cached statement's result"
  );
  Ok(())
}

#[test]
fn query_map_reuse_errors_after_a_referenced_column_is_renamed() -> TestResult {
  let conn = open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  let before = select_label(&conn, 1)?;
  assert_eq!(before.len(), 1);

  // Rename the `label` column SELECT_BY_ID's cached statement references.
  DbConnectionBlocking::execute_batch(&conn, "ALTER TABLE items RENAME COLUMN label TO tag")?;

  let result: Result<Vec<LabelRow>, DbError> = select_label(&conn, 1);
  match result {
    Err(DbError::Query(_)) => {},
    other => {
      return Err(
        format!("expected DbError::Query once a referenced column is renamed, got: {other:?}")
          .into(),
      );
    },
  }
  Ok(())
}

#[test]
fn query_map_error_does_not_poison_later_reuse() -> TestResult {
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
