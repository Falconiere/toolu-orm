//! The synchronized FTS5 schema both live sync suites migrate, and how they
//! read the resulting database back.
//!
//! `content_rowid` must be an integer column (an FTS5 rule), so the content
//! table keys on an integer primary key. Neither suite ever writes to the index
//! or issues a manual `rebuild`: every `MATCH` result is there because a
//! generated trigger put it there.

use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Migrations = (tempfile::TempDir, String);

/// An empty migrations directory, kept alive by the returned `TempDir`.
///
/// # Errors
///
/// Returns the I/O error, or a message when the path is not UTF-8.
pub fn migrations_dir() -> Result<Migrations, Box<dyn std::error::Error>> {
  let tmp = tempfile::tempdir()?;
  let dir = tmp.path().join("migrations");
  std::fs::create_dir(&dir)?;
  let path = dir.to_str().ok_or("non-UTF8 path")?.to_owned();
  Ok((tmp, path))
}

pub fn column(name: &str, column_type: ColumnType, primary_key: bool, not_null: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type,
    primary_key,
    not_null,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}

pub fn memories(body_not_null: bool) -> TableDef {
  TableDef {
    name: "memories".to_owned(),
    columns: vec![
      column("id", ColumnType::Integer, true, true),
      column("body", ColumnType::Text, false, body_not_null),
      column("note", ColumnType::Text, false, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }
}

pub fn memory_fts(tokenize: &str) -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .unindexed_column("note", ColumnType::Text)
    .tokenize(tokenize)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build()
}

/// The same index without the opt-in, for the suite that removes it.
pub fn memory_fts_unsynced(tokenize: &str) -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .unindexed_column("note", ColumnType::Text)
    .tokenize(tokenize)
    .content("memories")
    .content_rowid("id")
    .build()
}

pub fn registry(tokenize: &str, body_not_null: bool) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(body_not_null), memory_fts(tokenize)])
}

/// An in-memory libsql database.
///
/// # Errors
///
/// Returns the driver's connection error.
pub async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

/// First column of the first row, as an integer.
///
/// # Errors
///
/// Returns the driver's query error, or a message when the query is empty.
pub async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// How many indexed rows `term` matches.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn matches(
  conn: &LibsqlConnection,
  term: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT count(*) FROM memory_fts WHERE memory_fts MATCH '{term}'"),
  )
  .await
}

/// How many generated synchronization triggers the database holds.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn trigger_count(conn: &LibsqlConnection) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name LIKE 'toolu_fts5_%'",
  )
  .await
}

/// FTS5's own verdict that the index and the content table agree. It is what
/// would catch a trigger writing the wrong values rather than none at all.
///
/// # Errors
///
/// Returns the driver's error, which is what FTS5 reports a mismatch as.
pub async fn integrity_check(conn: &LibsqlConnection) -> TestResult {
  conn
    .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES('integrity-check');")
    .await?;
  Ok(())
}

/// Two content rows, indexed by the insert trigger alone.
///
/// # Errors
///
/// Returns the driver's error.
pub async fn seed(conn: &LibsqlConnection) -> TestResult {
  conn
    .execute_batch(
      "INSERT INTO memories (id, body, note) VALUES \
       (1, 'the athlete was running through zebrafish valley', 'first'), \
       (2, 'a quiet afternoon of reading', 'second');",
    )
    .await?;
  Ok(())
}

/// A third content row, written after a migration to prove the triggers came
/// back.
///
/// # Errors
///
/// Returns the driver's error.
pub async fn insert_heron(conn: &LibsqlConnection) -> TestResult {
  conn
    .execute_batch(
      "INSERT INTO memories (id, body, note) VALUES (3, 'a heron by the river', 'third');",
    )
    .await?;
  Ok(())
}
