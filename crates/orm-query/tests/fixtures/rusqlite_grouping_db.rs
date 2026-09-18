//! In-memory rusqlite grouping database seeded through `InsertBuilder`.

use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{
  ID, LABEL, PATH, SEED, SIZE_BYTES, SOURCES_DDL, SOURCES_SEED, SOURCE_ID, SOURCE_PK,
  SQLITE_FILES_DDL, STATUS,
};

/// One grouped row: a key and its count.
#[derive(FromRow, Debug, PartialEq)]
pub struct KeyCount {
  pub key: String,
  pub n: i64,
}

/// A grouped row keyed on two columns.
#[derive(FromRow, Debug, PartialEq)]
pub struct PairCount {
  pub source_id: String,
  pub status: String,
  pub n: i64,
}

/// Every aggregate over one group.
#[derive(FromRow, Debug, PartialEq)]
pub struct Totals {
  pub source_id: String,
  pub rows: i64,
  pub paths: i64,
  pub total: i64,
  pub largest: i64,
  pub smallest: i64,
  pub mean: f64,
}

/// A group whose mean came from integer division, which SQLite computes as an
/// integer. libsql's decoder does not coerce an INTEGER into `f64`, so the
/// Rust type has to match what the engine actually returned.
#[derive(FromRow, Debug, PartialEq)]
pub struct IntegerMean {
  pub source_id: String,
  pub total: i64,
  pub mean: i64,
}

/// An aggregate over a possibly empty set: NULL when no row matched.
#[derive(FromRow, Debug, PartialEq)]
pub struct MaybeTotal {
  pub total: Option<i64>,
  pub rows: i64,
}

/// A single projected text column.
#[derive(FromRow, Debug, PartialEq)]
pub struct JustPath {
  pub path: String,
}

/// Connects, creates both tables and inserts the seed rows.
///
/// # Errors
///
/// The underlying rusqlite or builder error.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(SQLITE_FILES_DDL, ())?;
  conn.execute(SOURCES_DDL, ())?;
  for (id, source_id, path, status, size_bytes) in SEED {
    InsertBuilder::new("source_files")
      .set(&ID, id)
      .set(&SOURCE_ID, source_id)
      .set(&PATH, path)
      .set(&STATUS, status)
      .set(&SIZE_BYTES, size_bytes)
      .execute(&conn)?;
  }
  for (id, label) in SOURCES_SEED {
    InsertBuilder::new("sources")
      .set(&SOURCE_PK, id)
      .set(&LABEL, label)
      .execute(&conn)?;
  }
  Ok(conn)
}

/// The paths of `rows`, for order-sensitive assertions.
pub fn paths(rows: &[JustPath]) -> Vec<&str> {
  rows.iter().map(|row| row.path.as_str()).collect()
}

/// `(key, count)` pairs, for order-sensitive assertions.
pub fn pairs(rows: &[KeyCount]) -> Vec<(&str, i64)> {
  rows.iter().map(|row| (row.key.as_str(), row.n)).collect()
}
