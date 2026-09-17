//! The blocking rusqlite runner rebuilds tables as safely as the async one.
//!
//! `run_migrate_blocking` has its own transaction and foreign-key handling, so
//! issue #85's reproduction is repeated here against a real in-memory rusqlite
//! connection with no tokio runtime in sight.

#[path = "fixtures/rebuild_registry.rs"]
pub mod rebuild_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate_blocking;
use toolu_orm_connection::{DbConnectionBlocking, RusqliteConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;

use rebuild_registry::{migrations_dir, registry_v1, registry_v2};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// One integer column, read positionally.
struct IntRow {
  value: i64,
}

impl FromRow for IntRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      value: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

/// One text column, read positionally.
struct TextRow {
  value: String,
}

impl FromRow for TextRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      value: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

fn scalar(conn: &RusqliteConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows: Vec<IntRow> = DbConnectionBlocking::query_map(conn, sql, vec![])?;
  Ok(rows.first().ok_or("scalar query returned no row")?.value)
}

fn fk_target(conn: &RusqliteConnection) -> Result<String, Box<dyn std::error::Error>> {
  let rows: Vec<TextRow> = DbConnectionBlocking::query_map(
    conn,
    "SELECT \"table\" FROM pragma_foreign_key_list('posts')",
    vec![],
  )?;
  Ok(
    rows
      .first()
      .ok_or("posts has no foreign key")?
      .value
      .clone(),
  )
}

/// v1 applied and seeded with a user and their post, then v2 applied.
fn rebuild_with_seeded_child(
  conn: &RusqliteConnection,
  dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  run_generate(&registry_v1(), dir, "init", Dialect::Sqlite)?;
  run_migrate_blocking(conn, dir, Dialect::Sqlite)?;
  conn.execute_batch(
    "INSERT INTO users (id, name, email, bio) VALUES ('u1', 'Ann', 'ann@x.io', 'hi'); \
     INSERT INTO posts (id, author_id, title) VALUES ('p1', 'u1', 'Hello')",
  )?;
  run_generate(&registry_v2(), dir, "tighten", Dialect::Sqlite)?;
  assert_eq!(run_migrate_blocking(conn, dir, Dialect::Sqlite)?, 1);
  Ok(())
}

#[test]
fn blocking_rebuild_keeps_cascading_child_rows_and_their_fk_target() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect()?;
  conn.execute_batch("PRAGMA foreign_keys = ON")?;

  rebuild_with_seeded_child(&conn, &dir)?;

  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM posts")?,
    1,
    "the rebuild cascade-deleted the child row"
  );
  assert_eq!(fk_target(&conn)?, "users");
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM pragma_foreign_key_check")?,
    0
  );
  assert_eq!(
    scalar(&conn, "PRAGMA foreign_keys")?,
    1,
    "foreign keys were left suspended"
  );
  Ok(())
}

#[test]
fn blocking_rebuild_leaves_foreign_keys_off_when_they_started_off() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect()?;
  conn.execute_batch("PRAGMA foreign_keys = OFF")?;
  assert_eq!(scalar(&conn, "PRAGMA foreign_keys")?, 0);

  rebuild_with_seeded_child(&conn, &dir)?;

  assert_eq!(
    scalar(&conn, "PRAGMA foreign_keys")?,
    0,
    "the migration switched foreign keys on behind the caller's back"
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts")?, 1);
  assert_eq!(fk_target(&conn)?, "users");
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_users_email'"
    )?,
    1,
    "the unchanged unique index was lost with the old table"
  );
  Ok(())
}
