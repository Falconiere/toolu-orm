//! MigrateError enum for migration runner failures.

use thiserror::Error;
use toolu_orm_connection::DbError;

#[derive(Debug, Error)]
#[non_exhaustive]
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

  /// A migration the database already applied is declared with a different
  /// hash than the one recorded when it ran: the journal entry (or the embedded
  /// entry) was rewritten after the fact, so the history the source describes is
  /// not the history this database has.
  ///
  /// Distinct from [`MigrateError::HashMismatch`], which reports a migration
  /// whose bytes no longer match the hash declared for them. The repairs
  /// differ: restore the journal entry here, restore the `.sql` file there.
  #[error(
    "migration {file} was applied with hash {recorded}, but the migration \
     source now declares {declared}"
  )]
  HistoryMismatch {
    file: String,
    recorded: String,
    declared: String,
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

  /// The migration suspended SQLite's foreign keys and left rows that violate
  /// one, so `PRAGMA foreign_key_check` reported them before the commit and
  /// the whole migration was rolled back.
  #[error("{file}: left {count} foreign key violation(s) behind; rolled back")]
  ForeignKeyViolation { file: String, count: i64 },

  /// Putting back the SQLite pragmas the runner borrowed failed. When the
  /// migration itself had already failed, `source` keeps that error whole, so
  /// a caller can still match on what actually went wrong.
  #[error("{message}")]
  PragmaRestore {
    message: String,
    #[source]
    source: Option<Box<MigrateError>>,
  },
}

pub(crate) fn map_db(err: &DbError) -> MigrateError {
  MigrateError::Database(err.to_string())
}
