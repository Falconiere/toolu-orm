//! DbError enum for connection-layer errors across all backends.

use toolu_orm_core::error::DbCoreError;

/// Errors produced by database connection operations.
///
/// This enum covers all failure modes across all backends (libsql, rusqlite, postgres).
/// Backend-specific errors are stringified into the appropriate variant.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
  /// Connection establishment or configuration failure.
  #[error("connection failed: {0}")]
  Connection(String),

  /// SQL execution or query failure.
  #[error("query failed: {0}")]
  Query(String),

  /// Transaction begin/commit/rollback failure.
  #[error("transaction failed: {0}")]
  Transaction(String),

  /// Connection pool exhaustion or management failure.
  #[error("pool error: {0}")]
  Pool(String),

  /// Row-to-struct mapping failure (type mismatch, missing column).
  #[error("row mapping failed: {0}")]
  RowMapping(String),
}

impl From<DbCoreError> for DbError {
  fn from(e: DbCoreError) -> Self {
    DbError::RowMapping(e.to_string())
  }
}
