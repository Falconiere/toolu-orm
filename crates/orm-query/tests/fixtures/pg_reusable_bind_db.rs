//! Live-Postgres fixture for the reusable-binding scenarios.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema; a missing server fails
//! the test rather than skipping it.

use toolu_orm_macros::FromRow;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{
  DST_ID, DST_KIND, EDGES_SEED, POSTGRES_DDL, REL, SRC_ID, SRC_KIND, TOUCHED_DDL, WEIGHT,
};

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

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, recreates `schema`, creates the tables and seeds them.
///
/// # Errors
///
/// Connection, DDL or insert failure.
pub async fn setup_db(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let conn_str = format!(
    "host={} port={} user={} password={} dbname={}",
    env_or("TEST_DB_HOST", "localhost"),
    env_or("TEST_DB_PORT", "5433"),
    env_or("TEST_DB_USER", "toolu"),
    env_or("TEST_DB_PASSWORD", "toolu"),
    env_or("TEST_DB_NAME", "toolu"),
  );
  let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;
  tokio::spawn(async move {
    if let Err(error) = connection.await {
      eprintln!("postgres connection task ended with error: {error}");
    }
  });
  client
    .batch_execute(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
       SET search_path TO {schema}; {POSTGRES_DDL}; {TOUCHED_DDL}"
    ))
    .await?;
  seed(&client).await?;
  Ok(client)
}

async fn seed(exec: &(impl Executor + Send + Sync)) -> Result<(), Box<dyn std::error::Error>> {
  for (rel, src_kind, src_id, dst_kind, dst_id, weight) in EDGES_SEED {
    InsertBuilder::new("edges")
      .set(&REL, *rel)
      .set(&SRC_KIND, *src_kind)
      .set(&SRC_ID, *src_id)
      .set(&DST_KIND, *dst_kind)
      .set(&DST_ID, *dst_id)
      .set(&WEIGHT, *weight)
      .execute(exec)
      .await?;
  }
  Ok(())
}

/// `(src_id, dst_id, weight)` triples, ordered, for a whole-table assertion.
pub fn edge_triples(rows: &[EdgeRow]) -> Vec<(&str, &str, i64)> {
  rows
    .iter()
    .map(|row| (row.src_id.as_str(), row.dst_id.as_str(), row.weight))
    .collect()
}
