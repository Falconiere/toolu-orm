//! In-memory libsql database for the alias/join scenario suite, seeded from the
//! shared `join_tables` fixture one statement at a time.

use crate::tables::{DDL, SEED};

/// # Errors
///
/// Returns the underlying libsql error if the connection, DDL or seed fails.
pub async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  for statement in DDL.iter().chain(SEED.iter()) {
    conn.execute(statement, ()).await?;
  }
  Ok(conn)
}
