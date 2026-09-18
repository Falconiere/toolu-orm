//! The error type for the SQLite maintenance and inspection surface.

use std::path::PathBuf;

/// A maintenance or inspection operation that did not complete.
///
/// SQLite's own failures keep their `rusqlite::Error` rather than being
/// stringified the way [`crate::DbError`] stringifies them: a caller that has
/// to tell "the snapshot destination already exists" from "the disk is full"
/// needs the result code, and only the original error carries it.
///
/// The other two variants are refusals this API makes before any statement
/// reaches the driver, so the connection is untouched when they are returned.
#[derive(Debug, thiserror::Error)]
pub enum MaintenanceError {
  /// SQLite refused the statement. The driver error is preserved whole,
  /// including its primary and extended result codes.
  #[error(transparent)]
  Sqlite(#[from] rusqlite::Error),

  /// A path that cannot be handed to SQLite, which takes filenames as UTF-8
  /// text. No statement was sent.
  #[error("database path is not valid UTF-8: {0:?}")]
  NonUtf8Path(PathBuf),

  /// An attachment identifier this API will not render. No statement was sent.
  ///
  /// Only an empty name and a name containing a NUL byte are refused; every
  /// other name, including one holding a `"`, a space, or a SQL keyword, is
  /// quoted and accepted.
  #[error("attachment schema name {name:?} is invalid: {reason}")]
  InvalidSchemaName {
    /// The name as it was given.
    name: String,
    /// Why it cannot be used.
    reason: &'static str,
  },
}
