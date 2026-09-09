//! Blocking baseline APIs against in-memory rusqlite with no tokio runtime.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{
  mark_applied_blocking, mark_applied_embedded_blocking, mark_applied_through_blocking,
  mark_applied_through_embedded_blocking, run_migrate_blocking, MigrateError,
};
use toolu_orm_connection::{DbConnectionBlocking, RusqliteConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations, INIT_SQL, POSTS_SQL};
use embedded_list::{honest, list, CREATE_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

fn adopted(dir: &str) -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  write_migrations(dir, &[("0001_init.sql", INIT_SQL)])?;
  let conn = connect()?;
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE users (id TEXT PRIMARY KEY)")?;
  Ok(conn)
}

fn has_table(conn: &RusqliteConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows: Vec<CountRow> = DbConnectionBlocking::query_map(
    conn,
    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
    vec![toolu_orm_core::value::Value::Text(name.to_owned())],
  )?;
  Ok(rows.first().map(|r| r.n).ok_or("no row")?)
}

fn recorded_hash(conn: &RusqliteConnection) -> Result<String, Box<dyn std::error::Error>> {
  let rows: Vec<HashRow> =
    DbConnectionBlocking::query_map(conn, "SELECT hash FROM _migrations ORDER BY id", vec![])?;
  Ok(rows.into_iter().next().map(|r| r.hash).ok_or("no hash")?)
}

struct CountRow {
  n: i64,
}
struct HashRow {
  hash: String,
}

impl toolu_orm_core::row::FromRow for CountRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["count(*)"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Ok(Self {
      n: row
        .get(0)
        .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

impl toolu_orm_core::row::FromRow for HashRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["hash"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Ok(Self {
      hash: row
        .get(0)
        .map_err(|e| toolu_orm_core::error::DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

#[test]
fn mark_applied_blocking_records_without_running_sql() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = adopted(&dir)?;

  assert_eq!(
    mark_applied_blocking(&conn, &dir, &["0001_init.sql"], Dialect::Sqlite)?,
    1
  );
  assert_eq!(has_table(&conn, "users")?, 1);
  assert_eq!(has_table(&conn, "audit")?, 0);
  assert_eq!(recorded_hash(&conn)?, compute_hash(INIT_SQL));

  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;
  assert_eq!(run_migrate_blocking(&conn, &dir, Dialect::Sqlite)?, 1);
  assert_eq!(has_table(&conn, "posts")?, 1);
  Ok(())
}

#[test]
fn mark_applied_through_blocking_records_prefix() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;
  let conn = connect()?;
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE users (id TEXT PRIMARY KEY); CREATE TABLE posts (id TEXT PRIMARY KEY);",
  )?;

  assert_eq!(
    mark_applied_through_blocking(&conn, &dir, "0002_posts.sql", Dialect::Sqlite)?,
    2
  );
  assert_eq!(has_table(&conn, "audit")?, 0);
  Ok(())
}

#[test]
fn unknown_name_is_not_in_journal() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  write_migrations(&dir, &[("0001_init.sql", INIT_SQL)])?;
  let conn = connect()?;

  let err =
    mark_applied_blocking(&conn, &dir, &["nope.sql"], Dialect::Sqlite).expect_err("unknown name");
  assert!(matches!(err, MigrateError::NotInJournal(_)));
  Ok(())
}

#[test]
fn embedded_baseline_blocking_twins() -> TestResult {
  let conn = connect()?;
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE users (id TEXT PRIMARY KEY)")?;
  let owned = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
  ];
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_embedded_blocking(&conn, &migrations, &["0001_init.sql"], Dialect::Sqlite)?,
    1
  );
  assert_eq!(has_table(&conn, "audit")?, 0);
  assert_eq!(
    mark_applied_through_embedded_blocking(&conn, &migrations, "0002_posts.sql", Dialect::Sqlite)?,
    1
  );
  Ok(())
}
