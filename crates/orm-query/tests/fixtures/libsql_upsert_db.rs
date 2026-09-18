//! In-memory libsql database for the upsert suite, with foreign keys on.

use super::schema::SQLITE_DDL;

/// Connects, turns foreign keys on and creates the three tables.
///
/// The pragma is set explicitly rather than trusted: `INSERT OR REPLACE`'s
/// cascade — the behavior these tests contrast `DO UPDATE` against — only
/// happens when it is enabled.
///
/// # Errors
///
/// The underlying libsql error if the connection, pragma or DDL fails.
pub async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let database = libsql::Builder::new_local(":memory:").build().await?;
  let conn = database.connect()?;
  conn.execute_batch("PRAGMA foreign_keys = ON;").await?;
  conn.execute_batch(SQLITE_DDL).await?;
  Ok(conn)
}

/// `PRAGMA foreign_keys`, so a test can assert it reads back `1`.
///
/// # Errors
///
/// The underlying libsql error, or a missing row.
pub async fn foreign_keys_enabled(
  conn: &libsql::Connection,
) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.query("PRAGMA foreign_keys", ()).await?;
  let row = rows
    .next()
    .await?
    .ok_or("PRAGMA foreign_keys returned no row")?;
  Ok(row.get::<i64>(0)?)
}
