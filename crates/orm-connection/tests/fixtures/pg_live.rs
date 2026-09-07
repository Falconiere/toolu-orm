//! Live-Postgres fixtures: per-test schema connection and a scalar count row.
//!
//! Wired into each `postgres_live_*_test.rs` binary with `#[path]`; every item
//! here is used by every binary that includes it (no dead code under
//! `-D warnings`).

use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const USERS_DDL: &str =
  "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL, age BIGINT)";

/// Opens a fresh pool and one connection whose `search_path` is a schema owned
/// by this test. The pool is returned so it outlives the connection.
pub async fn pg_schema_conn(
  schema: &str,
) -> Result<(PgDatabase, PgConnection), Box<dyn std::error::Error>> {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; SET search_path TO {schema}"
    ))
    .await?;
  Ok((db, conn))
}

pub struct CountRow {
  pub n: i64,
}

impl FromRow for CountRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["n"];

  fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError> {
    let n = row
      .try_get::<usize, i64>(0)
      .map_err(|e| DbCoreError::RowMapping(format!("column 0 (n): {e}")))?;
    Ok(Self { n })
  }

  fn from_libsql_row(_: &libsql::Row) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "CountRow is only decoded from Postgres rows".into(),
    ))
  }
}

pub async fn count_users(conn: &impl DbConnection) -> Result<i64, Box<dyn std::error::Error>> {
  let rows = conn
    .query_map::<CountRow>("SELECT count(*) AS n FROM users", vec![])
    .await?;
  Ok(rows.first().ok_or("count query returned no row")?.n)
}
