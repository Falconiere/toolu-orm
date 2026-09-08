//! The hidden `k = ?` conjunct that sizes a `vec0` KNN scan.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::Expr;
use crate::value::Value;

use super::require::require_sqlite;

const FEATURE: &str = "k";

/// `"k" = ?` for an explicit dialect — the hidden scan-size parameter.
///
/// This is not an ordinary filter: it must stay a top-level `AND` conjunct
/// next to the vector `MATCH`. Prefer `SelectBuilder::knn`, which pushes both;
/// call this only when assembling the pair by hand.
///
/// # Errors
///
/// - [`DbCoreError::Vec0UnsupportedDialect`] for [`Dialect::Postgres`]
/// - [`DbCoreError::Vec0InvalidArgument`] when `k <= 0`
pub fn k_eq_for(dialect: Dialect, k: i64) -> Result<Expr, DbCoreError> {
  require_sqlite(FEATURE, dialect)?;
  if k <= 0 {
    return Err(DbCoreError::Vec0InvalidArgument {
      feature: FEATURE.to_owned(),
      reason: format!("k must be a positive integer, got {k}"),
    });
  }
  Ok(Expr::comparison("\"k\"".to_owned(), "=", Value::Integer(k)))
}

/// [`k_eq_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`k_eq_for`].
pub fn k_eq(k: i64) -> Result<Expr, DbCoreError> {
  k_eq_for(Dialect::CURRENT, k)
}
