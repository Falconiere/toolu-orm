//! `query_map` (read path): reuse across changed parameters, schema-change
//! safety, and that one call's error never poisons a later reuse.

use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::QueryError;

use crate::db;
use crate::users::{insert_user, User};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SELECT_BY_ID: &str = "SELECT id, name, email, age FROM users WHERE id = ?1";

#[test]
fn query_map_reuses_cached_statement_across_changed_parameters() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;
  insert_user(&conn, "u2", "Bob", "bob@example.com", Some(25))?;

  let first: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  let second: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u2".to_owned())])?;

  let first_name = first
    .first()
    .map(|u| u.name.as_str())
    .ok_or("first call should find exactly one row")?;
  assert_eq!(first_name, "Alice");
  let second_name = second
    .first()
    .map(|u| u.name.as_str())
    .ok_or("second call should find exactly one row")?;
  assert_eq!(second_name, "Bob");
  Ok(())
}

#[test]
fn query_map_reuse_survives_unrelated_schema_change() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let before: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(before.len(), 1);

  // Unrelated schema change: creates a new table, does not touch `users`.
  conn.execute("CREATE TABLE unrelated (id INTEGER PRIMARY KEY)", ())?;

  let after: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(
    after, before,
    "an unrelated schema change must not change the cached statement's result"
  );
  Ok(())
}

#[test]
fn query_map_reuse_reports_error_after_table_drop() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let before: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(before.len(), 1);

  conn.execute("DROP TABLE users", ())?;

  let result: Result<Vec<User>, QueryError> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())]);
  match result {
    Err(QueryError::Driver(_)) => {},
    other => {
      return Err(
        format!("expected a driver error once the queried table is gone, got: {other:?}").into(),
      )
    },
  }
  Ok(())
}

#[test]
fn query_map_reuse_survives_added_column_on_the_queried_table() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let before: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(before.len(), 1);

  // A schema change to the queried table itself, but one that leaves every
  // column SELECT_BY_ID references untouched. See the module doc for why
  // this is safe: SQLite, not the adapter, revalidates and recompiles.
  conn.execute("ALTER TABLE users ADD COLUMN extra TEXT", ())?;

  let after: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(
    after, before,
    "ALTER TABLE ADD COLUMN on the queried table must not corrupt a cached statement's result"
  );
  Ok(())
}

#[test]
fn query_map_reuse_errors_after_a_referenced_column_is_renamed() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let before: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(before.len(), 1);

  // Rename a column SELECT_BY_ID's cached statement references by name.
  conn.execute("ALTER TABLE users RENAME COLUMN name TO full_name", ())?;

  let result: Result<Vec<User>, QueryError> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())]);
  match result {
    Err(QueryError::Driver(_)) => {},
    other => {
      return Err(
        format!("expected a driver error once a referenced column is renamed, got: {other:?}")
          .into(),
      )
    },
  }
  Ok(())
}

#[test]
fn query_map_error_does_not_poison_later_reuse() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  let first: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(first.len(), 1);

  let failing: Result<Vec<User>, QueryError> =
    Executor::query_map(&conn, "SELECT id FROM missing_table", vec![]);
  assert!(failing.is_err(), "querying a nonexistent table must fail");

  // Same SQL text this connection already ran successfully above.
  let second: Vec<User> =
    Executor::query_map(&conn, SELECT_BY_ID, vec![Value::Text("u1".to_owned())])?;
  assert_eq!(
    second, first,
    "a prior error on different SQL must not poison reuse of SQL that succeeded before"
  );
  Ok(())
}
