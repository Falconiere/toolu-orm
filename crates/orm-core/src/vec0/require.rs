//! Dialect gate shared by the vec0 query surface.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

/// Refuses anything but SQLite.
///
/// `vec0` KNN (`MATCH`, hidden `k`, synthesised `distance`) is sqlite-vec's.
/// Postgres pgvector uses different operators; translating silently would be
/// wrong. Rejecting here means no Postgres SQL containing a vec0 KNN piece
/// can be built at all.
pub(crate) fn require_sqlite(feature: &str, dialect: Dialect) -> Result<(), DbCoreError> {
  match dialect {
    Dialect::Sqlite => Ok(()),
    Dialect::Postgres => Err(DbCoreError::Vec0UnsupportedDialect {
      feature: feature.to_owned(),
      dialect: dialect.as_str(),
    }),
  }
}
