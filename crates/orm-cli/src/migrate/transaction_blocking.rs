//! Blocking transaction boundaries over [`DbConnectionBlocking`].

use toolu_orm_connection::DbConnectionBlocking;

use super::error::MigrateError;
use super::sql::{BEGIN, COMMIT, ROLLBACK};

pub(super) fn begin(conn: &impl DbConnectionBlocking) -> Result<(), MigrateError> {
  conn
    .execute_batch(BEGIN)
    .map_err(|e| MigrateError::Database(format!("begin transaction: {e}")))
}

pub(super) fn commit(conn: &impl DbConnectionBlocking) -> Result<(), MigrateError> {
  conn
    .execute_batch(COMMIT)
    .map_err(|e| MigrateError::Database(format!("commit transaction: {e}")))
}

/// Rolls back after `err`. A ROLLBACK that fails itself is appended so neither
/// error is lost.
pub(super) fn rollback_after(conn: &impl DbConnectionBlocking, err: MigrateError) -> MigrateError {
  match conn.execute_batch(ROLLBACK) {
    Ok(()) => err,
    Err(rollback_err) => {
      MigrateError::Database(format!("{err}; rollback also failed: {rollback_err}"))
    },
  }
}
