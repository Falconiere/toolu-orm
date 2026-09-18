//! In-memory libsql database for the reusable-binding scenarios.

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
/// The underlying libsql or builder error.
pub async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let database = libsql::Builder::new_local(":memory:").build().await?;
  let conn = database.connect()?;
  conn.execute(SQLITE_DDL, ()).await?;
  for (rel, src_kind, src_id, dst_kind, dst_id, weight) in EDGES_SEED {
    InsertBuilder::new("edges")
      .set(&REL, *rel)
      .set(&SRC_KIND, *src_kind)
      .set(&SRC_ID, *src_id)
      .set(&DST_KIND, *dst_kind)
      .set(&DST_ID, *dst_id)
      .set(&WEIGHT, *weight)
      .execute(&conn)
      .await?;
  }
  Ok(conn)
}

/// `(src_id, dst_id, weight)` triples, ordered, for a whole-table assertion.
pub fn edge_triples(rows: &[EdgeRow]) -> Vec<(&str, &str, i64)> {
  rows
    .iter()
    .map(|row| (row.src_id.as_str(), row.dst_id.as_str(), row.weight))
    .collect()
}
