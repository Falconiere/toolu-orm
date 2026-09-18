//! The `memories` table the scalar-expression scenarios run against, and the
//! four rows every driver seeds, so the SQLite and Postgres suites assert the
//! same data.
//!
//! The rows are chosen for the behaviors under test: two bodies that differ
//! only in a literal `%`, two that differ only in a literal `_`, three
//! `created_at` stamps of different sub-second precision around one cutoff,
//! and `last_accessed` absent on half the rows.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const ID: Column<Text> = Column::new("memories", "id");
pub const BODY: Column<Text> = Column::new("memories", "body");
pub const CREATED_AT: Column<Text> = Column::new("memories", "created_at");
pub const LAST_ACCESSED: Column<Text> = Column::new("memories", "last_accessed");
pub const ACCESS_COUNT: Column<Integer> = Column::new("memories", "access_count");

pub const MEMORY_COLUMNS: [&str; 5] = ["id", "body", "created_at", "last_accessed", "access_count"];

/// The cutoff AC-2 compares against: the same instant as `m1`, and earlier
/// than `m2` only once both are read as timestamps.
pub const CUTOFF: &str = "2026-09-18T10:00:00Z";

/// `(id, body, created_at, last_accessed, access_count)`.
pub const SEED: [(&str, &str, &str, Option<&str>, i64); 4] = [
  ("m1", "100% cotton", "2026-09-18T10:00:00Z", None, 0),
  (
    "m2",
    "100 percent cotton",
    "2026-09-18T10:00:00.123Z",
    Some("2026-09-19T08:00:00Z"),
    5,
  ),
  ("m3", "a_b", "2026-09-18T09:59:59.999999Z", None, 12),
  (
    "m4",
    "axb",
    "2026-09-17T10:00:00Z",
    Some("2026-09-16T08:00:00Z"),
    0,
  ),
];

/// SQLite DDL; Postgres swaps `INTEGER` for `BIGINT` in its own fixture.
pub const SQLITE_DDL: &str = "CREATE TABLE memories (id TEXT PRIMARY KEY, body TEXT NOT NULL, \
   created_at TEXT NOT NULL, last_accessed TEXT, access_count INTEGER NOT NULL)";
