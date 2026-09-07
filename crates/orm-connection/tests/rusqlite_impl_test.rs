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

struct LabelRow {
  label: String,
}

impl FromRow for LabelRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    let label: String = row
      .get(0)
      .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { label })
  }
}

/// Read the single scalar a one-row, one-column query returns.
async fn scalar(
  conn: &RusqliteConnection,
  sql: &str,
) -> Result<i64, toolu_orm_connection::DbError> {
  let rows: Vec<CountRow> = conn.query_map(sql, vec![]).await?;
  rows
    .first()
    .map(|row| row.count)
    .ok_or_else(|| toolu_orm_connection::DbError::Query(format!("no row for `{sql}`")))
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

/// The adopted connection keeps its own in-memory database: a table created
/// before adoption is still there afterwards. Reopening `:memory:` would give a
/// fresh, empty database and this query would fail with "no such table".
#[tokio::test]
async fn from_connection_adopts_existing_database() -> Result<(), toolu_orm_connection::DbError> {
  let raw = rusqlite::Connection::open_in_memory().unwrap();
  raw
    .execute_batch(
      "CREATE TABLE adopted (id INTEGER PRIMARY KEY, label TEXT NOT NULL);
       INSERT INTO adopted (id, label) VALUES (7, 'pre-existing');",
    )
    .unwrap();

  let conn = RusqliteConnection::from_connection(raw);

  let rows: Vec<LabelRow> = conn
    .query_map("SELECT label FROM adopted WHERE id = 7", vec![])
    .await?;
  assert_eq!(rows.len(), 1);
  assert_eq!(
    rows.first().map(|row| row.label.as_str()),
    Some("pre-existing")
  );
  Ok(())
}

/// Connection-scoped configuration applied before adoption survives it.
///
/// rusqlite 0.32's bundled SQLite is compiled with `SQLITE_DEFAULT_FOREIGN_KEYS`,
/// so `foreign_keys` reads `1` unless the caller turns it off; turning it off is
/// therefore the deviation worth pinning. The control connection, left at that
/// default, reads `1` through the same path, so a wrapper that reset or reopened
/// the connection instead of adopting it would fail here.
#[tokio::test]
async fn from_connection_preserves_connection_pragmas() -> Result<(), toolu_orm_connection::DbError>
{
  let configured = rusqlite::Connection::open_in_memory().unwrap();
  configured
    .execute_batch("PRAGMA foreign_keys = OFF;")
    .unwrap();
  let configured = RusqliteConnection::from_connection(configured);

  let control =
    RusqliteConnection::from_connection(rusqlite::Connection::open_in_memory().unwrap());

  assert_eq!(scalar(&configured, "PRAGMA foreign_keys").await?, 0);
  assert_eq!(scalar(&control, "PRAGMA foreign_keys").await?, 1);
  Ok(())
}

/// A temp database path that deletes itself, so a failed assertion or an early
/// `?` cannot leave the file behind.
struct TempDbPath(String);

impl TempDbPath {
  /// Process id plus a nanosecond timestamp: parallel test processes cannot
  /// collide on the same name.
  fn new() -> Self {
    let nanos = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap_or_default()
      .as_nanos();
    let path =
      std::env::temp_dir().join(format!("toolu-orm-open-{}-{nanos}.db", std::process::id()));
    Self(path.to_string_lossy().into_owned())
  }

  fn as_str(&self) -> &str {
    &self.0
  }
}

impl Drop for TempDbPath {
  fn drop(&mut self) {
    drop(std::fs::remove_file(&self.0));
  }
}

/// `open` still reaches a real file after being routed through
/// `from_connection`: a second wrapper reads what the first one wrote.
#[tokio::test]
async fn open_persists_to_a_file() -> Result<(), toolu_orm_connection::DbError> {
  let path = TempDbPath::new();

  let writer = RusqliteConnection::open(path.as_str()).await?;
  writer
    .execute_batch("CREATE TABLE persisted (count INTEGER NOT NULL)")
    .await?;
  writer
    .execute_sql(
      "INSERT INTO persisted (count) VALUES (?1)",
      vec![Value::Integer(99)],
    )
    .await?;
  drop(writer);

  let reader = RusqliteConnection::open(path.as_str()).await?;
  assert_eq!(scalar(&reader, "SELECT count FROM persisted").await?, 99);
  Ok(())
}
