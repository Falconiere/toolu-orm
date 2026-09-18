//! In-memory rusqlite database for the alias/join scenario suite, seeded from
//! the shared `join_tables` fixture.

use crate::tables::script;

/// # Errors
///
/// Returns the underlying rusqlite error if the connection, DDL or seed fails.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(&script())?;
  Ok(conn)
}
