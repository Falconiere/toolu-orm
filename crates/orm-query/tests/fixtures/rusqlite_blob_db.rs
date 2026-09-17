//! In-memory rusqlite database with a relation that carries a `BLOB` column.
//!
//! `owners` ← `files(owner_id)`; `files.payload` is nonempty, empty and NULL
//! across the seed, plus an owner with no files and a file with no owner.

/// The three payloads seeded for `f1`, `f2`, `f3`, in that order.
pub const PAYLOAD: &[u8] = &[0x01, 0x02, 0xFF, 0x00, 0x7F];

/// Opens the database, creates the schema and seeds it.
///
/// # Errors
///
/// Returns the underlying rusqlite error if the connection, DDL or seed fails.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(
    "CREATE TABLE owners (id TEXT PRIMARY KEY, name TEXT NOT NULL, avatar BLOB);
     CREATE TABLE files (
       id TEXT PRIMARY KEY,
       owner_id TEXT REFERENCES owners(id),
       payload BLOB
     );",
  )?;

  conn.execute(
    "INSERT INTO owners (id, name, avatar) VALUES (?1, ?2, ?3)",
    rusqlite::params!["o1", "Ann", PAYLOAD],
  )?;
  conn.execute(
    "INSERT INTO owners (id, name, avatar) VALUES ('o2', 'Bea', NULL)",
    (),
  )?;

  conn.execute(
    "INSERT INTO files (id, owner_id, payload) VALUES (?1, ?2, ?3)",
    rusqlite::params!["f1", "o1", PAYLOAD],
  )?;
  conn.execute(
    "INSERT INTO files (id, owner_id, payload) VALUES ('f2', 'o1', X'')",
    (),
  )?;
  conn.execute(
    "INSERT INTO files (id, owner_id, payload) VALUES ('f3', 'o1', NULL)",
    (),
  )?;
  conn.execute(
    "INSERT INTO files (id, owner_id, payload) VALUES ('f4', NULL, X'AB')",
    (),
  )?;
  Ok(conn)
}
