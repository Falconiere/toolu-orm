//! The two tables an `INSERT … SELECT` copy moves rows between, per engine.
//!
//! `legacy_index` is the older shape: it has no `rank`, so the copy has to
//! project one. `file_index` is the current shape, with `(repo, path)` unique
//! so a conflict mode has something to react to.

use toolu_orm_core::column::{Blob, Integer, Text};
use toolu_orm_core::query_column::Column;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const REPO: Column<Text> = Column::new("file_index", "repo");
pub const PATH: Column<Text> = Column::new("file_index", "path");
pub const BLOB_OID: Column<Blob> = Column::new("file_index", "blob_oid");
pub const INDEXED_AT: Column<Integer> = Column::new("file_index", "indexed_at");
pub const RANK: Column<Integer> = Column::new("file_index", "rank");

/// The source projection, in target-column order: `rank` is not a source
/// column, so it is a bound literal.
pub const SOURCE_COLUMNS: &[&str] = &["repo", "path", "blob_oid", "indexed_at"];

/// A byte string no text encoding round-trips.
pub const BLOB_BYTES: &[u8] = &[0x00, 0x10, 0xFF, 0x7F, 0x00, 0xC3];

pub const SQLITE_DDL: &str = "
CREATE TABLE file_index (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BLOB,
  indexed_at INTEGER,
  rank       INTEGER NOT NULL DEFAULT -1,
  PRIMARY KEY (repo, path)
);
CREATE TABLE legacy_index (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BLOB,
  indexed_at INTEGER,
  PRIMARY KEY (repo, path)
);
";

pub const POSTGRES_DDL: &str = "
CREATE TABLE file_index (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BYTEA,
  indexed_at BIGINT,
  rank       BIGINT NOT NULL DEFAULT -1,
  PRIMARY KEY (repo, path)
);
CREATE TABLE legacy_index (
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  blob_oid   BYTEA,
  indexed_at BIGINT,
  PRIMARY KEY (repo, path)
);
";

/// One landed row, decoded back out of the target.
#[derive(toolu_orm_macros::FromRow, Debug, PartialEq)]
pub struct FileRow {
  pub repo: String,
  pub path: String,
  pub blob_oid: Option<Vec<u8>>,
  pub indexed_at: Option<i64>,
  pub rank: i64,
}
