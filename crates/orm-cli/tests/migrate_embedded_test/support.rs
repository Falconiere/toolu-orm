//! The in-memory database, the runner call, and the raw-libsql readbacks every
//! module of this binary asserts through.

use toolu_orm_cli::migrate::{run_migrate_embedded, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;

use crate::embedded_list::{honest, list, OwnedMigration, CREATE_SQL};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
pub const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

pub async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

pub async fn migrate(
  conn: &LibsqlConnection,
  owned: &[OwnedMigration],
) -> Result<u32, MigrateError> {
  run_migrate_embedded(conn, &list(owned), Dialect::Sqlite).await
}

pub async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// 1 when the table exists, 0 when it does not.
pub async fn has_table(
  conn: &LibsqlConnection,
  name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '{name}'"),
  )
  .await
}

/// The recorded names in `_migrations.id` order — the order they were applied.
pub async fn recorded(conn: &LibsqlConnection) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query("SELECT name FROM _migrations ORDER BY id", ())
    .await?;
  let mut names = Vec::new();
  while let Some(row) = rows.next().await? {
    names.push(row.get::<String>(0)?);
  }
  Ok(names)
}

pub fn two_migrations() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
  ]
}
