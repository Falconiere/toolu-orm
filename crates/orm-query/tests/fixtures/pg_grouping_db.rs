//! Live-Postgres grouping fixture.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema; a missing server fails
//! the test rather than skipping it.
//!
//! `size_bytes` is `BIGINT`, which is what a bound `Value::Integer` arrives as.
//! That fixes what the aggregates return: `count`, `max` and `min` over a
//! `bigint` are `bigint`, but `sum(bigint)` and `avg(bigint)` are `numeric`,
//! which the row decoders do not map to a Rust scalar — so this suite asserts
//! the first three and leaves `SUM`/`AVG` to the two SQLite suites.

use toolu_orm_macros::FromRow;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{
  ID, LABEL, PATH, SEED, SIZE_BYTES, SOURCES_DDL, SOURCES_SEED, SOURCE_ID, SOURCE_PK, STATUS,
};

const FILES_DDL: &str = "CREATE TABLE source_files (id TEXT PRIMARY KEY, \
   source_id TEXT NOT NULL, path TEXT NOT NULL, status TEXT NOT NULL, \
   size_bytes BIGINT NOT NULL)";

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

/// The aggregates whose Postgres return type is `bigint`.
#[derive(FromRow, Debug, PartialEq)]
pub struct Totals {
  pub source_id: String,
  pub rows: i64,
  pub paths: i64,
  pub largest: i64,
  pub smallest: i64,
}

/// `MAX` over a possibly empty set: NULL when no row matched.
#[derive(FromRow, Debug, PartialEq)]
pub struct MaybeLargest {
  pub largest: Option<i64>,
  pub rows: i64,
}

/// A single projected text column.
#[derive(FromRow, Debug, PartialEq)]
pub struct JustPath {
  pub path: String,
}

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, recreates `schema`, creates both tables and seeds them.
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
       SET search_path TO {schema}; {FILES_DDL}; {SOURCES_DDL}"
    ))
    .await?;
  seed(&client).await?;
  Ok(client)
}

async fn seed(exec: &(impl Executor + Send + Sync)) -> Result<(), Box<dyn std::error::Error>> {
  for (id, source_id, path, status, size_bytes) in SEED {
    InsertBuilder::new("source_files")
      .set(&ID, id)
      .set(&SOURCE_ID, source_id)
      .set(&PATH, path)
      .set(&STATUS, status)
      .set(&SIZE_BYTES, size_bytes)
      .execute(exec)
      .await?;
  }
  for (id, label) in SOURCES_SEED {
    InsertBuilder::new("sources")
      .set(&SOURCE_PK, id)
      .set(&LABEL, label)
      .execute(exec)
      .await?;
  }
  Ok(())
}

/// The paths of `rows`, for order-sensitive assertions.
pub fn paths(rows: &[JustPath]) -> Vec<&str> {
  rows.iter().map(|row| row.path.as_str()).collect()
}

/// `(key, count)` pairs, for order-sensitive assertions.
pub fn pairs(rows: &[KeyCount]) -> Vec<(&str, i64)> {
  rows.iter().map(|row| (row.key.as_str(), row.n)).collect()
}
