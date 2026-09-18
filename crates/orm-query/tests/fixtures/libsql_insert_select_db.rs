//! In-memory libsql database for the `INSERT … SELECT` suite.
//!
//! One database holds both tables, so the `main` schema qualifier is what
//! exercises `TableRef::in_database` here — libsql has no `SqliteMaintenance`,
//! and a real cross-file `ATTACH` copy is the rusqlite suite's job.

use super::schema::{BLOB_BYTES, SQLITE_DDL};

/// Connects, creates both tables and seeds `legacy_index` with three rows: one
/// ordinary, one whose text and blob are NULL, and one with an empty blob.
///
/// # Errors
///
/// The underlying libsql error.
pub async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let database = libsql::Builder::new_local(":memory:").build().await?;
  let conn = database.connect()?;
  conn.execute_batch(SQLITE_DDL).await?;
  conn
    .execute(
      "INSERT INTO legacy_index (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, ?3, ?4)",
      libsql::params![
        "r1",
        "src/a.rs",
        libsql::Value::Blob(BLOB_BYTES.to_vec()),
        100_i64
      ],
    )
    .await?;
  conn
    .execute(
      "INSERT INTO legacy_index (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, NULL, NULL)",
      libsql::params!["r1", "src/b.rs"],
    )
    .await?;
  conn
    .execute(
      "INSERT INTO legacy_index (repo, path, blob_oid, indexed_at) VALUES (?1, ?2, ?3, ?4)",
      libsql::params!["r2", "src/c.rs", libsql::Value::Blob(Vec::new()), 300_i64],
    )
    .await?;
  Ok(conn)
}
