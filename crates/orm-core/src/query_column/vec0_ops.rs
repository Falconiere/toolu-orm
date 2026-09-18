//! Vector `MATCH` for a `vec0` KNN query.

use crate::column::Vector;
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::Expr;
use crate::value::Value;
use crate::vec0::require::require_sqlite as require_sqlite_vec;

use super::column::Column;

/// Vector `MATCH` for a `vec0` KNN query — the left half of
/// `embedding MATCH ? AND k = ?`.
///
/// Pair it with [`crate::vec0::k_eq`] (or, preferably, `SelectBuilder::knn`,
/// which pushes both as top-level conjuncts). Only [`Column<Vector>`] can
/// name the left-hand side: a non-vector column does not compile.
pub trait Vec0Ops {
  /// `"<table>"."<column>" MATCH ?` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::Vec0UnsupportedDialect`] for [`Dialect::Postgres`].
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, query: V) -> Result<Expr, DbCoreError>;

  /// [`Vec0Ops::matches_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Vec0Ops::matches_for`].
  fn matches<V: Into<Value>>(&self, query: V) -> Result<Expr, DbCoreError> {
    self.matches_for(Dialect::CURRENT, query)
  }
}

impl Vec0Ops for Column<Vector> {
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, query: V) -> Result<Expr, DbCoreError> {
    require_sqlite_vec("MATCH", dialect)?;
    Ok(Expr::match_target(self.qualified(), query.into()))
  }
}
