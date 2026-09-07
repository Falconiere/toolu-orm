//! Transaction boundaries shared by the migration runner and the baseline.

use toolu_orm_connection::DbConnection;

use super::error::MigrateError;

pub(super) async fn begin(conn: &impl DbConnection) -> Result<(), MigrateError> {
  conn
    .execute_batch("BEGIN")
    .await
    .map_err(|e| MigrateError::Database(format!("begin transaction: {e}")))
}

pub(super) async fn commit(conn: &impl DbConnection) -> Result<(), MigrateError> {
  conn
    .execute_batch("COMMIT")
    .await
    .map_err(|e| MigrateError::Database(format!("commit transaction: {e}")))
}

/// Rolls back after `err`. A ROLLBACK that fails itself (connection lost) is
/// appended to the message so neither error is lost.
pub(super) async fn rollback_after(conn: &impl DbConnection, err: MigrateError) -> MigrateError {
  match conn.execute_batch("ROLLBACK").await {
    Ok(()) => err,
    Err(rollback_err) => {
      MigrateError::Database(format!("{err}; rollback also failed: {rollback_err}"))
    },
  }
}
