//! Prepared-statement reuse against a real in-memory rusqlite database
//! (rusqlite-only lane, synchronous): `Executor for rusqlite::Connection`'s
//! `query_map`/`execute_sql` now go through `Connection::prepare_cached`
//! instead of `prepare`/`execute`. These suites prove that reuse behaves
//! exactly like the uncached path across changed parameters, a schema change,
//! an error followed by reuse, and a row-mapping failure (issue #88).

#[path = "fixtures/rusqlite_db.rs"]
pub mod db;
#[path = "fixtures/rusqlite_users.rs"]
pub mod users;

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::QueryError;
use users::{insert_user, User, USER_COLUMNS};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SELECT_BY_ID: &str = "SELECT id, name, email, age FROM users WHERE id = ?1";

/// Decodes column 0 as an integer regardless of its actual type, so selecting
/// a text column (`name`) through it forces a rusqlite type-mismatch error,
/// which `Executor::query_map` must surface as `QueryError::RowMapping`. No
/// decoded value is kept -- only whether decoding itself succeeds matters.
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

#[test]
fn query_map_decode_failure_is_row_mapping_error() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  // `name` is TEXT; decoding column 0 as i64 must fail.
  let result: Result<Vec<MismatchedRow>, QueryError> = Executor::query_map(
    &conn,
    "SELECT name FROM users WHERE id = ?1",
    vec![Value::Text("u1".to_owned())],
  );

  match result {
    Err(QueryError::RowMapping { .. }) => {},
    other => return Err(format!("expected RowMapping, got: {other:?}").into()),
  }
  Ok(())
}

// ── execute_sql (write path) ────────────────────────────────────────────────

#[test]
fn execute_sql_reuses_cached_statement_across_changed_parameters() -> TestResult {
  let conn = db::setup_db()?;
  const INSERT: &str = "INSERT INTO users (id, name, email, age) VALUES (?1, ?2, ?3, ?4)";

  let affected_first = Executor::execute_sql(
    &conn,
    INSERT,
    vec![
      Value::Text("u1".to_owned()),
      Value::Text("Ann".to_owned()),
      Value::Text("ann@example.com".to_owned()),
      Value::Integer(30),
    ],
  )?;
  let affected_second = Executor::execute_sql(
    &conn,
    INSERT,
    vec![
      Value::Text("u2".to_owned()),
      Value::Text("Bea".to_owned()),
      Value::Text("bea@example.com".to_owned()),
      Value::Integer(31),
    ],
  )?;

  assert_eq!(affected_first, 1);
  assert_eq!(affected_second, 1);

  let rows: Vec<User> = Executor::query_map(
    &conn,
    &format!("SELECT {} FROM users ORDER BY id", USER_COLUMNS.join(", ")),
    vec![],
  )?;
  match rows.as_slice() {
    [ann, bea] => {
      assert_eq!(ann.name, "Ann");
      assert_eq!(bea.name, "Bea");
    },
    other => return Err(format!("expected exactly two rows, got: {other:?}").into()),
  }
  Ok(())
}
