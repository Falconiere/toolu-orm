//! [`PgVectorOps`] — distance constructors on [`Column<Vector>`].

use crate::column::Vector;
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::query_column::Column;

use super::distance::{distance_for, DistanceOp, PgVectorDistance};

/// pgvector distance operators restricted to a [`Column<Vector>`].
///
/// Pair with `SelectBuilder::column_expr` / `order_by` / `limit`. This is not
/// [`crate::vec0`] KNN and does not share APIs with it.
pub trait PgVectorOps {
  /// `"col" <op> '[…]'::vector` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::PgVectorUnsupportedDialect`] for [`Dialect::Sqlite`];
  /// [`DbCoreError::PgVectorInvalidArgument`] for non-finite embedding elements.
  fn distance_for(
    &self,
    dialect: Dialect,
    op: DistanceOp,
    query: &[f32],
  ) -> Result<PgVectorDistance, DbCoreError>;

  /// [`PgVectorOps::distance_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn distance(&self, op: DistanceOp, query: &[f32]) -> Result<PgVectorDistance, DbCoreError> {
    self.distance_for(Dialect::CURRENT, op, query)
  }

  /// L2 (`<->`) for an explicit dialect.
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn l2_distance_for(
    &self,
    dialect: Dialect,
    query: &[f32],
  ) -> Result<PgVectorDistance, DbCoreError> {
    self.distance_for(dialect, DistanceOp::L2, query)
  }

  /// L2 against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn l2_distance(&self, query: &[f32]) -> Result<PgVectorDistance, DbCoreError> {
    self.l2_distance_for(Dialect::CURRENT, query)
  }

  /// Cosine (`<=>`) for an explicit dialect.
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn cosine_distance_for(
    &self,
    dialect: Dialect,
    query: &[f32],
  ) -> Result<PgVectorDistance, DbCoreError> {
    self.distance_for(dialect, DistanceOp::Cosine, query)
  }

  /// Cosine against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn cosine_distance(&self, query: &[f32]) -> Result<PgVectorDistance, DbCoreError> {
    self.cosine_distance_for(Dialect::CURRENT, query)
  }

  /// Negative inner product (`<#>`) for an explicit dialect.
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn neg_inner_product_for(
    &self,
    dialect: Dialect,
    query: &[f32],
  ) -> Result<PgVectorDistance, DbCoreError> {
    self.distance_for(dialect, DistanceOp::NegInnerProduct, query)
  }

  /// Negative inner product against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`PgVectorOps::distance_for`].
  fn neg_inner_product(&self, query: &[f32]) -> Result<PgVectorDistance, DbCoreError> {
    self.neg_inner_product_for(Dialect::CURRENT, query)
  }
}

impl PgVectorOps for Column<Vector> {
  fn distance_for(
    &self,
    dialect: Dialect,
    op: DistanceOp,
    query: &[f32],
  ) -> Result<PgVectorDistance, DbCoreError> {
    distance_for(dialect, self, op, query)
  }
}
