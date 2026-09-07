//! Mutation scenarios against an in-memory libsql database (libsql-only
//! lane): `InsertBuilder::or_replace()`/`or_ignore()` conflict handling, and
//! `Value` round-trips through the `Executor`.

#[path = "fixtures/libsql_db.rs"]
mod db;
#[path = "fixtures/libsql_users.rs"]
mod users;

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use users::{insert_user, User, USER_AGE, USER_COLUMNS, USER_EMAIL, USER_ID, USER_NAME};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── Upsert ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn or_replace_replaces_conflicting_row() -> TestResult {
  let conn = db::setup_db().await?;
  insert_user(&conn, "bystander", "Zoe", "zoe@example.com", Some(22)).await?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Ann")
    .set(&USER_EMAIL, "ann@example.com")
    .set(&USER_AGE, 30_i64)
    .or_replace()
    .conflict_columns(&["id"])
    .execute(&conn)
    .await?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Bea")
    .set(&USER_EMAIL, "bea@example.com")
    .set(&USER_AGE, 31_i64)
    .or_replace()
    .conflict_columns(&["id"])
    .execute(&conn)
    .await?;

  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .fetch_all(&conn)
    .await?;

  assert_eq!(rows.len(), 2, "replace must not touch unrelated rows");
  let replaced: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_ID.eq("u1"))
    .fetch_one(&conn)
    .await?;
  assert_eq!(replaced.name, "Bea");
  Ok(())
}

#[tokio::test]
async fn or_ignore_keeps_original_row() -> TestResult {
  let conn = db::setup_db().await?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Ann")
    .set(&USER_EMAIL, "ann@example.com")
    .set(&USER_AGE, 30_i64)
    .or_ignore()
    .execute(&conn)
    .await?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Bea")
    .set(&USER_EMAIL, "bea@example.com")
    .set(&USER_AGE, 31_i64)
    .or_ignore()
    .execute(&conn)
    .await?;

  let user: User = SelectBuilder::new("users")
    .columns_raw(&USER_COLUMNS)
    .filter(USER_ID.eq("u1"))
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.name, "Ann");
  Ok(())
}

// ── Value round-trips ────────────────────────────────────────────────────────

async fn setup_probe() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn
    .execute(
      "CREATE TABLE probe (id INTEGER PRIMARY KEY, val_int INTEGER, val_real REAL, val_text TEXT, val_blob BLOB)",
      (),
    )
    .await?;
  Ok(conn)
}

#[tokio::test]
async fn value_null_round_trips_as_none() -> TestResult {
  let conn = setup_probe().await?;
  conn
    .execute_sql(
      "INSERT INTO probe (id, val_int) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Null],
    )
    .await?;

  let mut rows = conn
    .query("SELECT val_int FROM probe WHERE id = 1", ())
    .await?;
  let row = rows.next().await?.ok_or("missing row")?;
  let val: Option<i64> = row.get(0)?;
  assert_eq!(val, None);
  Ok(())
}

#[tokio::test]
async fn value_integer_round_trips_i64_max() -> TestResult {
  let conn = setup_probe().await?;
  conn
    .execute_sql(
      "INSERT INTO probe (id, val_int) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Integer(i64::MAX)],
    )
    .await?;

  let mut rows = conn
    .query("SELECT val_int FROM probe WHERE id = 1", ())
    .await?;
  let row = rows.next().await?.ok_or("missing row")?;
  let val: i64 = row.get(0)?;
  assert_eq!(val, i64::MAX);
  Ok(())
}

#[tokio::test]
async fn value_real_round_trips() -> TestResult {
  let conn = setup_probe().await?;
  conn
    .execute_sql(
      "INSERT INTO probe (id, val_real) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Real(1.5)],
    )
    .await?;

  let mut rows = conn
    .query("SELECT val_real FROM probe WHERE id = 1", ())
    .await?;
  let row = rows.next().await?.ok_or("missing row")?;
  let val: f64 = row.get(0)?;
  assert!((val - 1.5).abs() < f64::EPSILON);
  Ok(())
}

#[tokio::test]
async fn value_text_round_trips_unicode() -> TestResult {
  let conn = setup_probe().await?;
  conn
    .execute_sql(
      "INSERT INTO probe (id, val_text) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Text("héllo".to_owned())],
    )
    .await?;

  let mut rows = conn
    .query("SELECT val_text FROM probe WHERE id = 1", ())
    .await?;
  let row = rows.next().await?.ok_or("missing row")?;
  let val: String = row.get(0)?;
  assert_eq!(val, "héllo");
  Ok(())
}

#[tokio::test]
async fn value_blob_round_trips_byte_exact() -> TestResult {
  let conn = setup_probe().await?;
  conn
    .execute_sql(
      "INSERT INTO probe (id, val_blob) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Blob(vec![0, 255, 7])],
    )
    .await?;

  let mut rows = conn
    .query("SELECT val_blob FROM probe WHERE id = 1", ())
    .await?;
  let row = rows.next().await?.ok_or("missing row")?;
  let val: Vec<u8> = row.get(0)?;
  assert_eq!(val, vec![0, 255, 7]);
  Ok(())
}
