//! Live-Postgres database for the alias/join scenario suite.
//!
//! Reads the same `TEST_DB_*` variables as `PgConfig::for_test` (defaults
//! `localhost`, `5433`, `toolu`/`toolu`, db `toolu`) and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema, and an absent server is
//! a hard failure rather than a skip.

use crate::tables::script;

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, drops and recreates `schema`, sets `search_path`, then creates and
/// seeds the fixture tables.
///
/// # Errors
///
/// Connection, DDL or seed failure.
pub async fn client(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
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
    if let Err(e) = connection.await {
      eprintln!("postgres connection task ended with error: {e}");
    }
  });
  client
    .batch_execute(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
       SET search_path TO {schema}; {}",
      script()
    ))
    .await?;
  Ok(client)
}
