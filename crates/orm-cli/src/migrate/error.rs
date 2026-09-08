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

  /// An embedded migration list names the same migration more than once, so
  /// there is no single SQL body for that name.
  #[error("duplicate migration name(s) in the embedded list: {0}")]
  DuplicateMigration(String),

  /// A statement needed a SQLite module the connection has not loaded
  /// (`vec0`, …). The fix is to register the extension on the connection
  /// before `run_migrate`, not to change the migration.
  #[error(
    "{file}: this database has no \"{module}\" module — load the extension that \
     provides it on the connection before running migrations"
  )]
  MissingExtension { file: String, module: String },
}

pub(crate) fn map_db(err: &DbError) -> MigrateError {
  MigrateError::Database(err.to_string())
}
