//! The schema-qualified pragma reads shared by the trait and the guard.
//!
//! Each read takes `Option<&str>`: `None` is the main database, `Some(name)` an
//! attachment. The identifier is *not* rendered here — `pragma_query` and
//! `pragma_query_value` build the statement through rusqlite's own
//! `Sql::push_identifier`, which double-quotes the schema and doubles any
//! interior `"`, so an attachment called `we"ird` is addressed correctly
//! without this module writing a single character of SQL.
//!
//! These statements deliberately skip the connection's prepared-statement
//! cache (`docs/scenarios/prepared-statement-cache.md`): a pragma read is a
//! one-shot administrative call, and caching it would evict a hot query for no
//! gain.

use super::error::MaintenanceError;
use super::integrity::IntegrityReport;
use super::storage::StorageStats;

/// Read a one-row, one-integer pragma.
fn read_int(
  conn: &rusqlite::Connection,
  schema: Option<&str>,
  pragma: &str,
) -> Result<i64, MaintenanceError> {
  Ok(conn.pragma_query_value(schema, pragma, |row| row.get(0))?)
}

pub(super) fn page_count(
  conn: &rusqlite::Connection,
  schema: Option<&str>,
) -> Result<i64, MaintenanceError> {
  read_int(conn, schema, "page_count")
}

pub(super) fn page_size(
  conn: &rusqlite::Connection,
  schema: Option<&str>,
) -> Result<i64, MaintenanceError> {
  read_int(conn, schema, "page_size")
}

pub(super) fn storage_stats(
  conn: &rusqlite::Connection,
  schema: Option<&str>,
) -> Result<StorageStats, MaintenanceError> {
  Ok(StorageStats {
    page_count: page_count(conn, schema)?,
    page_size: page_size(conn, schema)?,
  })
}

pub(super) fn quick_check(
  conn: &rusqlite::Connection,
  schema: Option<&str>,
) -> Result<IntegrityReport, MaintenanceError> {
  let mut messages = Vec::new();
  // A badly corrupted database can make SQLite abort partway through the
  // pragma. That surfaces as the driver error it is, rather than as a short
  // report that would read like a clean result.
  conn.pragma_query(schema, "quick_check", |row| {
    messages.push(row.get::<_, String>(0)?);
    Ok(())
  })?;
  Ok(IntegrityReport::new(messages))
}
