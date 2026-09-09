//! Blocking `run_migrate` against in-memory rusqlite with no tokio runtime.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;

use toolu_orm_cli::migrate::run_migrate_blocking;
use toolu_orm_connection::{DbConnectionBlocking, RusqliteConnection};
use toolu_orm_core::dialect::Dialect;

use baseline_dir::{migrations_dir, write_migrations, INIT_SQL, POSTS_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

fn recorded(conn: &RusqliteConnection) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let rows: Vec<NameRow> =
    DbConnectionBlocking::query_map(conn, "SELECT name FROM _migrations ORDER BY id", vec![])?;
  Ok(rows.into_iter().map(|r| r.name).collect())
}

struct CountRow {
  n: i64,
}
struct NameRow {
  name: String,
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

impl toolu_orm_core::row::FromRow for NameRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Ok(Self {
      name: row
        .get(0)
        .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

#[test]
fn run_migrate_blocking_applies_journaled_migrations() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;
  let conn = connect()?;

  assert_eq!(run_migrate_blocking(&conn, &dir, Dialect::Sqlite)?, 2);
  assert_eq!(has_table(&conn, "users")?, 1);
  assert_eq!(has_table(&conn, "audit")?, 1);
  assert_eq!(has_table(&conn, "posts")?, 1);
  assert_eq!(
    recorded(&conn)?,
    vec!["0001_init.sql".to_owned(), "0002_posts.sql".to_owned()]
  );

  assert_eq!(run_migrate_blocking(&conn, &dir, Dialect::Sqlite)?, 0);
  Ok(())
}
