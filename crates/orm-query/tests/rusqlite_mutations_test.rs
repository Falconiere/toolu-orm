//! Mutation scenarios against an in-memory rusqlite database (rusqlite-only
//! lane, synchronous): `InsertBuilder::or_replace()`/`or_ignore()` conflict
//! handling, and `Value` round-trips through the `Executor`.

#[path = "fixtures/rusqlite_db.rs"]
pub mod db;
#[path = "fixtures/rusqlite_users.rs"]
pub mod users;

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use users::{insert_user, User, USER_AGE, USER_COLUMNS, USER_EMAIL, USER_ID, USER_NAME};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── Upsert ───────────────────────────────────────────────────────────────────

#[test]
fn or_replace_replaces_conflicting_row() -> TestResult {
  let conn = db::setup_db()?;
  insert_user(&conn, "bystander", "Zoe", "zoe@example.com", Some(22))?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Ann")
    .set(&USER_EMAIL, "ann@example.com")
    .set(&USER_AGE, 30_i64)
    .or_replace()
    .conflict_columns(&["id"])
    .execute(&conn)?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Bea")
    .set(&USER_EMAIL, "bea@example.com")
    .set(&USER_AGE, 31_i64)
    .or_replace()
    .conflict_columns(&["id"])
    .execute(&conn)?;

  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_all(&conn)?;

  assert_eq!(rows.len(), 2, "replace must not touch unrelated rows");
  let replaced: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_ID.eq("u1"))
    .fetch_one(&conn)?;
  assert_eq!(replaced.name, "Bea");
  Ok(())
}

#[test]
fn or_ignore_keeps_original_row() -> TestResult {
  let conn = db::setup_db()?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Ann")
    .set(&USER_EMAIL, "ann@example.com")
    .set(&USER_AGE, 30_i64)
    .or_ignore()
    .execute(&conn)?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Bea")
    .set(&USER_EMAIL, "bea@example.com")
    .set(&USER_AGE, 31_i64)
    .or_ignore()
    .execute(&conn)?;

  let user: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_ID.eq("u1"))
    .fetch_one(&conn)?;

  assert_eq!(user.name, "Ann");
  Ok(())
}

// ── Value round-trips ────────────────────────────────────────────────────────

fn setup_probe() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(
    "CREATE TABLE probe (id INTEGER PRIMARY KEY, val_int INTEGER, val_real REAL, val_text TEXT, val_blob BLOB)",
    (),
  )?;
  Ok(conn)
}

#[test]
fn value_null_round_trips_as_none() -> TestResult {
  let conn = setup_probe()?;
  conn.execute_sql(
    "INSERT INTO probe (id, val_int) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Null],
  )?;

  let val: Option<i64> = conn.query_row("SELECT val_int FROM probe WHERE id = 1", [], |row| {
    row.get(0)
  })?;
  assert_eq!(val, None);
  Ok(())
}

#[test]
fn value_integer_round_trips_i64_max() -> TestResult {
  let conn = setup_probe()?;
  conn.execute_sql(
    "INSERT INTO probe (id, val_int) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Integer(i64::MAX)],
  )?;

  let val: i64 = conn.query_row("SELECT val_int FROM probe WHERE id = 1", [], |row| {
    row.get(0)
  })?;
  assert_eq!(val, i64::MAX);
  Ok(())
}

#[test]
fn value_real_round_trips() -> TestResult {
  let conn = setup_probe()?;
  conn.execute_sql(
    "INSERT INTO probe (id, val_real) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Real(1.5)],
  )?;

  let val: f64 = conn.query_row("SELECT val_real FROM probe WHERE id = 1", [], |row| {
    row.get(0)
  })?;
  assert!((val - 1.5).abs() < f64::EPSILON);
  Ok(())
}

#[test]
fn value_text_round_trips_unicode() -> TestResult {
  let conn = setup_probe()?;
  conn.execute_sql(
    "INSERT INTO probe (id, val_text) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Text("héllo".to_owned())],
  )?;

  let val: String = conn.query_row("SELECT val_text FROM probe WHERE id = 1", [], |row| {
    row.get(0)
  })?;
  assert_eq!(val, "héllo");
  Ok(())
}

#[test]
fn value_blob_round_trips_byte_exact() -> TestResult {
  let conn = setup_probe()?;
  conn.execute_sql(
    "INSERT INTO probe (id, val_blob) VALUES (?1, ?2)",
    vec![Value::Integer(1), Value::Blob(vec![0, 255, 7])],
  )?;

  let val: Vec<u8> = conn.query_row("SELECT val_blob FROM probe WHERE id = 1", [], |row| {
    row.get(0)
  })?;
  assert_eq!(val, vec![0, 255, 7]);
  Ok(())
}
