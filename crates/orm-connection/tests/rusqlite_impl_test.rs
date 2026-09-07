// Single-backend shape: `FromRow` exposes `from_row(&rusqlite::Row)` only when
// rusqlite is the sole driver feature on orm-core (the rusqlite-only lane).
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::DbConnection;
use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

struct CountRow {
  count: i64,
}

impl FromRow for CountRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["count"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    let count: i64 = row
      .get(0)
      .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { count })
  }
}

#[tokio::test]
async fn execute_batch_creates_table() -> Result<(), toolu_orm_connection::DbError> {
  let conn = RusqliteConnection::open_in_memory().await?;
  conn
    .execute_batch("CREATE TABLE test_batch (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
    .await?;
  Ok(())
}

#[tokio::test]
async fn execute_sql_inserts_row() -> Result<(), toolu_orm_connection::DbError> {
  let conn = RusqliteConnection::open_in_memory().await?;
  conn
    .execute_batch("CREATE TABLE test_insert (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
    .await?;

  let affected = conn
    .execute_sql(
      "INSERT INTO test_insert (id, name) VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Text("alice".to_owned())],
    )
    .await?;

  assert_eq!(affected, 1);
  Ok(())
}

#[tokio::test]
async fn query_map_returns_rows() -> Result<(), toolu_orm_connection::DbError> {
  let conn = RusqliteConnection::open_in_memory().await?;
  conn
    .execute_batch("CREATE TABLE test_query (count INTEGER NOT NULL)")
    .await?;
  conn
    .execute_sql(
      "INSERT INTO test_query (count) VALUES (?1)",
      vec![Value::Integer(42)],
    )
    .await?;

  let rows: Vec<CountRow> = conn
    .query_map("SELECT count FROM test_query", vec![])
    .await?;

  assert_eq!(rows.len(), 1);
  let row = rows
    .first()
    .ok_or_else(|| toolu_orm_connection::DbError::Query("missing row".to_owned()))?;
  assert_eq!(row.count, 42);
  Ok(())
}

#[tokio::test]
async fn execute_sql_returns_error_on_bad_sql() -> Result<(), toolu_orm_connection::DbError> {
  let conn = RusqliteConnection::open_in_memory().await?;
  let result = conn
    .execute_sql(
      "INSERT INTO nonexistent (x) VALUES (?1)",
      vec![Value::Integer(1)],
    )
    .await;
  assert!(result.is_err());
  Ok(())
}
