//! MigrateError enum for migration runner failures.

use thiserror::Error;
use toolu_orm_connection::DbError;

#[derive(Debug, Error)]
pub enum MigrateError {
  #[error("database error: {0}")]
  Database(String),

  #[error("failed to read migrations directory: {0}")]
  ReadDir(String),

  #[error("failed to read migration file: {0}")]
  ReadFile(String),

  #[error("migration {file} has been modified (expected {expected}, got {actual})")]
  HashMismatch {
    file: String,
    expected: String,
    actual: String,
  },

  /// A baseline named a migration the journal does not list, so there is no
  /// hash to record for it.
  #[error("no journal entry for migration(s): {0}")]
  NotInJournal(String),
}

pub(crate) fn map_db(err: &DbError) -> MigrateError {
  MigrateError::Database(err.to_string())
}
