//! The adopted database and the raw-libsql readbacks this binary asserts
//! through.

use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};

use crate::embedded_list::{honest, OwnedMigration, CREATE_SQL};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
pub const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

/// A database already carrying `users` (a prior migration system put it there)
/// plus an embedded list whose first entry would recreate it and also create
/// `audit`.
pub async fn adopted_db() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = Database::init_local(":memory:").await?.connect()?;
  conn
    .execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY)")
    .await?;
  Ok(conn)
}

pub fn init_list() -> Vec<OwnedMigration> {
  vec![honest("0001_init.sql", CREATE_SQL)]
}

pub fn three_list() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
    honest("0003_c.sql", THIRD_SQL),
  ]
}

pub async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

pub async fn text(
  conn: &LibsqlConnection,
  sql: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("query returned no row")?;
  Ok(row.get::<String>(0)?)
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
