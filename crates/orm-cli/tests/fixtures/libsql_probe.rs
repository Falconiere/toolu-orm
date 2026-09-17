//! Reading SQLite schema and pragma state off a real libsql connection.
//!
//! The rebuild tests assert on the live database rather than on the generated
//! SQL, so they need `PRAGMA` and `sqlite_master` reads that the `DbConnection`
//! trait does not model. Shared by the `sqlite_rebuild_*_libsql_test` and
//! `migrate_history_*_test` binaries.

use toolu_orm_connection::{Database, LibsqlConnection};

/// An in-memory libsql database.
///
/// # Errors
///
/// Returns the driver's connection error.
pub async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

/// Runs one statement that returns no rows.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn exec(conn: &LibsqlConnection, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
  conn.inner_conn().execute(sql, ()).await?;
  Ok(())
}

/// First column of the first row, as an integer.
///
/// # Errors
///
/// Returns the driver's query error, or a message when the query is empty.
pub async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// First column of the first row, as text; `None` when there is no row.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn text(
  conn: &LibsqlConnection,
  sql: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  match rows.next().await? {
    Some(row) => Ok(Some(row.get::<String>(0)?)),
    None => Ok(None),
  }
}

/// Whether a table of this name exists. The name is bound, never interpolated.
///
/// # Errors
///
/// Returns the driver's query error, or a message when the query is empty.
pub async fn has_table(
  conn: &LibsqlConnection,
  name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query(
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
      libsql::params![name],
    )
    .await?;
  let row = rows.next().await?.ok_or("table query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// Column names of `table`, in declaration order.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn column_names(
  conn: &LibsqlConnection,
  table: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query(&format!("PRAGMA table_info({table})"), ())
    .await?;
  let mut names = Vec::new();
  while let Some(row) = rows.next().await? {
    names.push(row.get::<String>(1)?);
  }
  Ok(names)
}

/// Whether foreign keys are enforced on this connection right now.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn foreign_keys_on(conn: &LibsqlConnection) -> Result<bool, Box<dyn std::error::Error>> {
  Ok(scalar(conn, "PRAGMA foreign_keys").await? == 1)
}

/// Rows anywhere in the database that violate a foreign key.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn foreign_key_violations(
  conn: &LibsqlConnection,
) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(conn, "SELECT count(*) FROM pragma_foreign_key_check").await
}

/// The table `posts.author_id` currently points at.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn posts_fk_target(
  conn: &LibsqlConnection,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
  text(
    conn,
    "SELECT \"table\" FROM pragma_foreign_key_list('posts')",
  )
  .await
}

/// Seeds one user and one of their posts.
///
/// # Errors
///
/// Returns the driver's query error.
pub async fn seed_user_and_post(conn: &LibsqlConnection) -> Result<(), Box<dyn std::error::Error>> {
  exec(
    conn,
    "INSERT INTO users (id, name, email, bio) VALUES ('u1', 'Ann', 'ann@x.io', 'hi')",
  )
  .await?;
  exec(
    conn,
    "INSERT INTO posts (id, author_id, title) VALUES ('p1', 'u1', 'Hello')",
  )
  .await
}
