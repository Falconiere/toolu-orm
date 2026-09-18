//! Live-Postgres fixture for the upsert suite.
//!
//! orm-query has no dependency on orm-connection, so this reads the same
//! `TEST_DB_*` variables as `PgConfig::for_test` and opens a raw
//! `tokio_postgres::Client`. Every test owns a schema; a missing server fails
//! the test rather than skipping it.

use super::schema::POSTGRES_DDL;

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// Connects, recreates `schema` and creates the three tables.
///
/// # Errors
///
/// Connection or DDL failure.
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
       SET search_path TO {schema}; {POSTGRES_DDL}"
    ))
    .await?;
  Ok(client)
}
