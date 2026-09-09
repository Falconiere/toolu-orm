//! Dialect gate shared by the pgvector query surface.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

/// Refuses anything but Postgres.
///
/// pgvector distance operators (`<->` / `<=>` / `<#>`) are a different model
/// from sqlite-vec KNN. Rejecting here means no SQLite SQL containing these
/// forms can be built from this module.
pub(crate) fn require_postgres(feature: &str, dialect: Dialect) -> Result<(), DbCoreError> {
  match dialect {
    Dialect::Postgres => Ok(()),
    Dialect::Sqlite => Err(DbCoreError::PgVectorUnsupportedDialect {
      feature: feature.to_owned(),
      dialect: dialect.as_str(),
    }),
  }
}
