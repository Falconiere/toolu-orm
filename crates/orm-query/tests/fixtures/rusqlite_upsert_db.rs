//! In-memory rusqlite database for the upsert suite, with foreign keys on.

use super::schema::SQLITE_DDL;

/// Opens the database, creates the three tables and turns foreign keys on.
///
/// The pragma is set explicitly rather than trusted: `INSERT OR REPLACE`'s
/// cascade — the behavior these tests contrast `DO UPDATE` against — only
/// happens when it is enabled.
///
/// # Errors
///
/// The underlying rusqlite error if the connection, pragma or DDL fails.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch("PRAGMA foreign_keys = ON;")?;
  conn.execute_batch(SQLITE_DDL)?;
  Ok(conn)
}

/// `PRAGMA foreign_keys`, so a test can assert it reads back `1`.
///
/// # Errors
///
/// The underlying rusqlite error.
pub fn foreign_keys_enabled(conn: &rusqlite::Connection) -> Result<i64, rusqlite::Error> {
  conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))
}
