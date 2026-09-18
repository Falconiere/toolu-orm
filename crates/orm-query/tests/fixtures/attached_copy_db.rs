//! Two real on-disk SQLite databases shaped like comemory's runtime rebuild:
//! a current store and an older one whose `indexed_files` lacks a column.
//!
//! The source really is a *different file*, reached only through
//! `ATTACH DATABASE`, so the copy under test crosses a database boundary rather
//! than moving rows inside one.

use std::path::Path;

use toolu_orm_core::column::{Blob, Integer, Text};
use toolu_orm_core::query_column::Column;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const REPO: Column<Text> = Column::new("indexed_files", "repo");
pub const PATH: Column<Text> = Column::new("indexed_files", "path");
pub const BLOB_OID: Column<Blob> = Column::new("indexed_files", "blob_oid");
pub const INDEXED_AT: Column<Integer> = Column::new("indexed_files", "indexed_at");
/// Present on the current store, absent from the older one.
pub const RANK: Column<Integer> = Column::new("indexed_files", "rank");

pub const TARGET_COLUMNS: &[&str] = &["repo", "path", "blob_oid", "indexed_at", "rank"];

/// The current store: `(repo, path)` is unique, and `rank` exists.
const TARGET_DDL: &str = "
CREATE TABLE indexed_files (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BLOB,
  indexed_at INTEGER,
  rank       INTEGER NOT NULL DEFAULT -1,
  PRIMARY KEY (repo, path)
);
CREATE TABLE sync_state (repo TEXT PRIMARY KEY, cursor TEXT);
";

/// The older store: same table, **no** `rank`, and one extra table the current
/// store does not have — so a catalogue read can tell the two apart.
const SOURCE_DDL: &str = "
CREATE TABLE indexed_files (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BLOB,
  indexed_at INTEGER,
  PRIMARY KEY (repo, path)
);
CREATE TABLE legacy_notes (id TEXT PRIMARY KEY);
";

/// One row of the current store, decoded back out for assertions.
#[derive(Debug, PartialEq)]
pub struct IndexedFile {
  pub repo: String,
  pub path: String,
  pub blob_oid: Option<Vec<u8>>,
  pub indexed_at: Option<i64>,
  pub rank: i64,
}

/// A byte string no text encoding round-trips: a NUL, a high byte, and `0xFF`.
///
/// It is the blob fidelity of the copy that is under test, so the bytes are
/// chosen to break any path that goes through a `String`.
pub const BLOB_BYTES: &[u8] = &[0x00, 0x10, 0xFF, 0x7F, 0x00, 0xC3];

/// Create the older store at `path` and seed it with three rows: one ordinary,
/// one whose `blob_oid` is a real blob, and one whose text and blob are NULL.
///
/// # Errors
///
/// The underlying rusqlite error.
pub fn seed_source(path: &Path) -> Result<(), rusqlite::Error> {
  let conn = rusqlite::Connection::open(path)?;
  conn.execute_batch(SOURCE_DDL)?;
  conn.execute(
    "INSERT INTO indexed_files (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, ?3, ?4)",
    rusqlite::params!["r1", "src/a.rs", BLOB_BYTES, 100_i64],
  )?;
  conn.execute(
    "INSERT INTO indexed_files (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, NULL, NULL)",
    rusqlite::params!["r1", "src/b.rs"],
  )?;
  conn.execute(
    "INSERT INTO indexed_files (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, ?3, ?4)",
    rusqlite::params!["r2", "src/c.rs", Vec::<u8>::new(), 300_i64],
  )?;
  Ok(())
}

/// Open the current store at `path`, empty.
///
/// # Errors
///
/// The underlying rusqlite error.
pub fn open_target(path: &Path) -> Result<rusqlite::Connection, rusqlite::Error> {
  let conn = rusqlite::Connection::open(path)?;
  conn.execute_batch(TARGET_DDL)?;
  Ok(conn)
}

/// Every row of the current store's `indexed_files`, ordered by key.
///
/// Read with the driver directly rather than through a builder, so what the
/// copy landed is observed independently of the code under test.
///
/// # Errors
///
/// The underlying rusqlite error.
pub fn target_rows(conn: &rusqlite::Connection) -> Result<Vec<IndexedFile>, rusqlite::Error> {
  let mut stmt = conn.prepare(
    "SELECT repo, path, blob_oid, indexed_at, rank FROM main.indexed_files ORDER BY repo, path",
  )?;
  let rows = stmt.query_map([], |row| {
    Ok(IndexedFile {
      repo: row.get(0)?,
      path: row.get(1)?,
      blob_oid: row.get(2)?,
      indexed_at: row.get(3)?,
      rank: row.get(4)?,
    })
  })?;
  rows.collect()
}
