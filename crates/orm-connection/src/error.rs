//! DbError enum for connection-layer errors across all backends.

use toolu_orm_core::error::DbCoreError;

/// Errors produced by database connection operations.
///
/// This enum covers connection failures and structured capability refusals across
/// libsql, rusqlite, PostgreSQL, and Lance. Driver errors carry their diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
  /// A structured operation requires a guarantee this backend cannot provide.
  #[error("unsupported capability {capability_name} on {backend_name}: {alternative}",
    capability_name = .capability.as_str(), backend_name = .backend.as_str(),
    alternative = .capability.alternative())]
  UnsupportedCapability {
    /// Backend selected by the connection.
    backend: toolu_orm_core::dialect::Dialect,
    /// Missing guarantee with a stable name and explicit alternative.
    capability: crate::capability::Capability,
  },

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
