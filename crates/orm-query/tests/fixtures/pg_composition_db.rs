//! Live-Postgres fixture for the query-composition scenarios.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema; a missing server fails
//! the test rather than skipping it.
//!
//! The walk's `depth` is `CAST(0 AS BIGINT)`, so `MIN(depth)` stays `bigint`
//! and decodes into an `i64`; a bare `0` would make the column `integer` and
//! the decode would fail.

use toolu_orm_macros::FromRow;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{
  ALL_DDL, EDGES_SEED, EDGE_DST_ID, EDGE_DST_KIND, EDGE_SRC_ID, EDGE_SRC_KIND, ITEMS_SEED, ITEM_ID,
  ITEM_OWNER_ID, OWNERS_SEED, OWNER_ID, SEED_BATCH, SEED_ID, SEED_KIND, SYMBOLS_SEED, SYMBOL_ID,
  SYMBOL_PATH, SYMBOL_REPO, VEC_NOTE, VEC_SEED, VEC_SYMBOL_ID, WALK_SEEDS,
};

/// One node of a walk and the shallowest depth it was reached at.
#[derive(FromRow, Debug, PartialEq)]
pub struct WalkRow {
  pub id: String,
  pub depth: i64,
}

/// A single projected id.
#[derive(FromRow, Debug, PartialEq)]
pub struct IdRow {
  pub id: String,
}

/// An item and the number of owner rows its key matches.
#[derive(FromRow, Debug, PartialEq)]
pub struct OwnerCountRow {
  pub id: String,
  pub owner_rows: i64,
}

/// A surviving `code_vec` row.
#[derive(FromRow, Debug, PartialEq)]
pub struct VecRow {
  pub symbol_id: String,
  pub note: String,
}

/// One part of a `regexp_split_to_table` expansion. Postgres names a
/// set-returning function's single output column after the `AS` alias.
#[derive(FromRow, Debug, PartialEq)]
pub struct PartRow {
  pub parts: String,
}

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, recreates `schema`, creates every table and seeds them.
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
       SET search_path TO {schema}; {}",
      ALL_DDL.join("; ")
    ))
    .await?;
  seed(&client).await?;
  Ok(client)
}

async fn seed(exec: &(impl Executor + Send + Sync)) -> Result<(), Box<dyn std::error::Error>> {
  for (src_kind, src_id, dst_kind, dst_id) in EDGES_SEED {
    InsertBuilder::new("edges")
      .set(&EDGE_SRC_KIND, src_kind)
      .set(&EDGE_SRC_ID, src_id)
      .set(&EDGE_DST_KIND, dst_kind)
      .set(&EDGE_DST_ID, dst_id)
      .execute(exec)
      .await?;
  }
  for (batch, kind, id) in WALK_SEEDS {
    InsertBuilder::new("walk_seeds")
      .set(&SEED_BATCH, batch)
      .set(&SEED_KIND, kind)
      .set(&SEED_ID, id)
      .execute(exec)
      .await?;
  }
  for owner in OWNERS_SEED {
    let insert = InsertBuilder::new("owners");
    match owner {
      Some(id) => insert.set(&OWNER_ID, id).execute(exec).await?,
      None => insert.set_null(&OWNER_ID).execute(exec).await?,
    };
  }
  for (id, owner_id) in ITEMS_SEED {
    let insert = InsertBuilder::new("items").set(&ITEM_ID, id);
    match owner_id {
      Some(owner) => insert.set(&ITEM_OWNER_ID, owner).execute(exec).await?,
      None => insert.set_null(&ITEM_OWNER_ID).execute(exec).await?,
    };
  }
  for (id, repo, path) in SYMBOLS_SEED {
    InsertBuilder::new("code_symbols")
      .set(&SYMBOL_ID, id)
      .set(&SYMBOL_REPO, repo)
      .set(&SYMBOL_PATH, path)
      .execute(exec)
      .await?;
  }
  for (symbol_id, note) in VEC_SEED {
    InsertBuilder::new("code_vec")
      .set(&VEC_SYMBOL_ID, symbol_id)
      .set(&VEC_NOTE, note)
      .execute(exec)
      .await?;
  }
  Ok(())
}

/// `(id, depth)` pairs, for order-sensitive assertions.
pub fn walk_pairs(rows: &[WalkRow]) -> Vec<(&str, i64)> {
  rows
    .iter()
    .map(|row| (row.id.as_str(), row.depth))
    .collect()
}

/// The ids of `rows`, for order-sensitive assertions.
pub fn ids(rows: &[IdRow]) -> Vec<&str> {
  rows.iter().map(|row| row.id.as_str()).collect()
}
