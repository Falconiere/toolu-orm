//! Live-Postgres `memories` fixture for the scalar-expression parity suite.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema; a missing server fails
//! the test rather than skipping it.

use toolu_orm_macros::FromRow;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{ACCESS_COUNT, BODY, CREATED_AT, ID, LAST_ACCESSED, SEED};

/// The SQLite DDL with `BIGINT` for the counter, which is what a bound
/// `Value::Integer` binds as on Postgres.
const DDL: &str = "CREATE TABLE memories (id TEXT PRIMARY KEY, body TEXT NOT NULL, \
   created_at TEXT NOT NULL, last_accessed TEXT, access_count BIGINT NOT NULL)";

/// Field order matches `MEMORY_COLUMNS`, which is how the derive decodes.
#[derive(FromRow, Debug, Clone, PartialEq)]
pub struct Memory {
  pub id: String,
  pub body: String,
  pub created_at: String,
  pub last_accessed: Option<String>,
  pub access_count: i64,
}

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, recreates `schema`, creates `memories` and seeds the four rows.
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
       SET search_path TO {schema}; {DDL}"
    ))
    .await?;
  seed(&client).await?;
  Ok(client)
}

async fn seed(exec: &(impl Executor + Send + Sync)) -> Result<(), Box<dyn std::error::Error>> {
  for (id, body, created_at, last_accessed, access_count) in SEED {
    let builder = InsertBuilder::new("memories")
      .set(&ID, id)
      .set(&BODY, body)
      .set(&CREATED_AT, created_at)
      .set(&ACCESS_COUNT, access_count);
    let builder = match last_accessed {
      Some(stamp) => builder.set(&LAST_ACCESSED, stamp),
      None => builder.set_null(&LAST_ACCESSED),
    };
    builder.execute(exec).await?;
  }
  Ok(())
}

/// The ids of `rows`, for order-sensitive assertions.
pub fn ids(rows: &[Memory]) -> Vec<&str> {
  rows.iter().map(|row| row.id.as_str()).collect()
}
