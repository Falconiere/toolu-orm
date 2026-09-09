//! Blocking embedded migrate against in-memory rusqlite with no tokio runtime.

#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{run_migrate_embedded_blocking, MigrateError};
use toolu_orm_connection::{DbConnectionBlocking, RusqliteConnection};
use toolu_orm_core::dialect::Dialect;

use embedded_list::{honest, list, tampered, CREATE_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

fn has_table(conn: &RusqliteConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows: Vec<CountRow> = DbConnectionBlocking::query_map(
    conn,
    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
    vec![toolu_orm_core::value::Value::Text(name.to_owned())],
  )?;
  Ok(rows.first().map(|r| r.n).ok_or("no row")?)
}

struct CountRow {
  n: i64,
}

impl toolu_orm_core::row::FromRow for CountRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Ok(Self {
      n: row
        .get(0)
        .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

#[test]
fn run_migrate_embedded_blocking_applies_list() -> TestResult {
  let conn = connect()?;
  let owned = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
  ];

  assert_eq!(
    run_migrate_embedded_blocking(&conn, &list(&owned), Dialect::Sqlite)?,
    2
  );
  assert_eq!(has_table(&conn, "users")?, 1);
  assert_eq!(has_table(&conn, "posts")?, 1);
  assert_eq!(
    run_migrate_embedded_blocking(&conn, &list(&owned), Dialect::Sqlite)?,
    0
  );
  Ok(())
}

#[test]
fn tampered_hash_fails_without_runtime() -> TestResult {
  let conn = connect()?;
  let owned = vec![tampered(
    "0001_init.sql",
    CREATE_SQL,
    "CREATE TABLE users (id TEXT PRIMARY KEY);",
  )];

  let err = run_migrate_embedded_blocking(&conn, &list(&owned), Dialect::Sqlite)
    .expect_err("tampered hash must fail");
  assert!(matches!(err, MigrateError::HashMismatch { .. }));
  Ok(())
}
