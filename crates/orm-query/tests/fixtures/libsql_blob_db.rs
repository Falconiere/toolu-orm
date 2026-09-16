//! In-memory libsql database with a relation that carries a `BLOB` column.
//!
//! Mirrors `rusqlite_blob_db.rs`: `owners` ← `files(owner_id)` with nonempty,
//! empty and NULL payloads, an owner with no files and a file with no owner.

/// The payload seeded for `f1` and for `o1`'s avatar.
pub const PAYLOAD: &[u8] = &[0x01, 0x02, 0xFF, 0x00, 0x7F];

/// Opens the database, creates the schema and seeds it.
pub(crate) async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn
    .execute_batch(
      "CREATE TABLE owners (id TEXT PRIMARY KEY, name TEXT NOT NULL, avatar BLOB);
       CREATE TABLE files (
         id TEXT PRIMARY KEY,
         owner_id TEXT REFERENCES owners(id),
         payload BLOB
       );",
    )
    .await?;

  conn
    .execute(
      "INSERT INTO owners (id, name, avatar) VALUES (?1, ?2, ?3)",
      libsql::params!["o1", "Ann", PAYLOAD.to_vec()],
    )
    .await?;
  conn
    .execute(
      "INSERT INTO owners (id, name, avatar) VALUES ('o2', 'Bea', NULL)",
      (),
    )
    .await?;
  conn
    .execute(
      "INSERT INTO files (id, owner_id, payload) VALUES (?1, ?2, ?3)",
      libsql::params!["f1", "o1", PAYLOAD.to_vec()],
    )
    .await?;
  conn
    .execute_batch(
      "INSERT INTO files (id, owner_id, payload) VALUES ('f2', 'o1', X'');
       INSERT INTO files (id, owner_id, payload) VALUES ('f3', 'o1', NULL);
       INSERT INTO files (id, owner_id, payload) VALUES ('f4', NULL, X'AB');",
    )
    .await?;
  Ok(conn)
}
