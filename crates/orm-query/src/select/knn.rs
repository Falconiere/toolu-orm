//! [`SelectBuilder::knn`] — the vec0 KNN form as top-level WHERE conjuncts.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, Vec0Ops};
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0;

use super::SelectBuilder;

impl SelectBuilder {
  /// Add top-level `embedding MATCH ? AND k = ?` vec0 predicates.
  ///
  /// `k` controls the scan before other filters; oversample for a full page.
  /// Execution requires the `sqlite-vec` extension.
  ///
  /// # Errors
  ///
  /// - [`DbCoreError::Vec0UnsupportedDialect`] for [`Dialect::Postgres`] or
  ///   [`Dialect::Lance`]
  /// - [`DbCoreError::Vec0InvalidArgument`] when `k <= 0` or `.knn` was
  ///   already applied on this builder
  pub fn knn_for<V: Into<Value>>(
    mut self,
    dialect: Dialect,
    column: &Column<Vector>,
    query: V,
    k: i64,
  ) -> Result<Self, DbCoreError> {
    if self.knn_applied {
      return Err(DbCoreError::Vec0InvalidArgument {
        feature: "knn".to_owned(),
        reason: "knn was already applied on this builder".to_owned(),
      });
    }
    let hit = column.matches_for(dialect, query)?;
    let k_expr = vec0::k_eq_for(dialect, k)?;
    self.filters.push(hit);
    self.filters.push(k_expr);
    self.knn_applied = true;
    Ok(self)
  }

  /// [`Self::knn_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Self::knn_for`].
  pub fn knn<V: Into<Value>>(
    self,
    column: &Column<Vector>,
    query: V,
    k: i64,
  ) -> Result<Self, DbCoreError> {
    self.knn_for(Dialect::CURRENT, column, query, k)
  }
}
