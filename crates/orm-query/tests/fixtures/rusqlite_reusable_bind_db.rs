//! In-memory rusqlite database for the reusable-binding scenarios.

use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{DST_ID, DST_KIND, EDGES_SEED, REL, SQLITE_DDL, SRC_ID, SRC_KIND, WEIGHT};

/// The aggregated co-change weight a query reports.
#[derive(FromRow, Debug, PartialEq)]
pub struct WeightRow {
  pub weight: i64,
}

/// One edge endpoint.
#[derive(FromRow, Debug, PartialEq)]
pub struct IdRow {
  pub id: String,
}

/// One row of the `INSERT … SELECT` target.
#[derive(FromRow, Debug, PartialEq)]
pub struct TouchedRow {
  pub id: String,
  pub note: String,
}

/// One whole edge row, for reading a table back after a mutation.
#[derive(FromRow, Debug, PartialEq)]
pub struct EdgeRow {
  pub src_id: String,
  pub dst_id: String,
  pub weight: i64,
}

/// Creates `edges` and inserts the seed rows.
///
/// # Errors
///
/// The underlying rusqlite or builder error.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(SQLITE_DDL, ())?;
  for (rel, src_kind, src_id, dst_kind, dst_id, weight) in EDGES_SEED {
    InsertBuilder::new("edges")
      .set(&REL, *rel)
      .set(&SRC_KIND, *src_kind)
      .set(&SRC_ID, *src_id)
      .set(&DST_KIND, *dst_kind)
      .set(&DST_ID, *dst_id)
      .set(&WEIGHT, *weight)
      .execute(&conn)?;
  }
  Ok(conn)
}

/// `(id, note)` pairs, for an order-sensitive assertion.
pub fn touched_pairs(rows: &[TouchedRow]) -> Vec<(&str, &str)> {
  rows
    .iter()
    .map(|row| (row.id.as_str(), row.note.as_str()))
    .collect()
}

/// `(src_id, dst_id, weight)` triples, ordered, for a whole-table assertion.
pub fn edge_triples(rows: &[EdgeRow]) -> Vec<(&str, &str, i64)> {
  rows
    .iter()
    .map(|row| (row.src_id.as_str(), row.dst_id.as_str(), row.weight))
    .collect()
}
