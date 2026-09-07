//! In-memory rusqlite connection with `users`/`posts` DDL, shared by every
//! rusqlite execution-scenario binary (mutations, reads, relational).

/// # Errors
///
/// Returns the underlying rusqlite error if the connection or DDL fails.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(
    "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, age INTEGER)",
    (),
  )?;
  conn.execute(
    "CREATE TABLE posts (id TEXT PRIMARY KEY, author_id TEXT REFERENCES users(id), title TEXT NOT NULL)",
    (),
  )?;
  Ok(conn)
}
