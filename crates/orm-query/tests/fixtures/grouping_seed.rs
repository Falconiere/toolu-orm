//! The two tables the DISTINCT/GROUP BY scenarios run against, and the rows
//! every driver seeds, so the SQLite and Postgres suites assert the same data.
//!
//! The rows are chosen so each acceptance criterion has a *discriminating*
//! answer. `s1` holds six rows over three distinct paths, so a page of
//! distinct paths differs from a page of raw rows. `status` splits into three
//! groups of 4 / 2 / 1, so ordering by the count is unambiguous and
//! `HAVING COUNT(*) > 1` keeps exactly two. `(source_id, status)` makes four
//! groups. `s1`'s sizes total 165 over six rows — an average of 27.5, which
//! integer division could not produce.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const ID: Column<Text> = Column::new("source_files", "id");
pub const SOURCE_ID: Column<Text> = Column::new("source_files", "source_id");
pub const PATH: Column<Text> = Column::new("source_files", "path");
pub const STATUS: Column<Text> = Column::new("source_files", "status");
pub const SIZE_BYTES: Column<Integer> = Column::new("source_files", "size_bytes");

pub const SOURCE_PK: Column<Text> = Column::new("sources", "id");
pub const LABEL: Column<Text> = Column::new("sources", "label");

/// `(id, source_id, path, status, size_bytes)`.
pub const SEED: [(&str, &str, &str, &str, i64); 7] = [
  ("f1", "s1", "src/a.rs", "indexed", 10),
  ("f2", "s1", "src/a.rs", "indexed", 20),
  ("f3", "s1", "src/b.rs", "pending", 30),
  ("f4", "s1", "src/c.rs", "failed", 40),
  ("f5", "s2", "src/a.rs", "indexed", 50),
  ("f6", "s1", "src/b.rs", "indexed", 60),
  ("f7", "s1", "src/c.rs", "failed", 5),
];

/// `(id, label)` — the join partner, so a grouping key can come from an
/// aliased *joined* relation.
pub const SOURCES_SEED: [(&str, &str); 2] = [("s1", "primary"), ("s2", "secondary")];

/// SQLite DDL; Postgres swaps `INTEGER` for `BIGINT` in its own fixture.
pub const SQLITE_FILES_DDL: &str = "CREATE TABLE source_files (id TEXT PRIMARY KEY, \
   source_id TEXT NOT NULL, path TEXT NOT NULL, status TEXT NOT NULL, \
   size_bytes INTEGER NOT NULL)";

pub const SOURCES_DDL: &str = "CREATE TABLE sources (id TEXT PRIMARY KEY, label TEXT NOT NULL)";
