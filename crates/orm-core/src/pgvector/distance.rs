//! Distance expression: `"table"."col" <-> '[…]'::vector` (and siblings).

use crate::column::Vector;
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::OrderBy;
use crate::query_column::Column;

use super::literal::vector_literal;
use super::require::require_postgres;

/// Which pgvector distance operator to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceOp {
  /// Euclidean — `<->`.
  L2,
  /// Cosine distance — `<=>`.
  Cosine,
  /// Negative inner product — `<#>` (ASC = nearest).
  NegInnerProduct,
}

impl DistanceOp {
  /// The SQL operator token.
  #[must_use]
  pub const fn sql_op(self) -> &'static str {
    match self {
      Self::L2 => "<->",
      Self::Cosine => "<=>",
      Self::NegInnerProduct => "<#>",
    }
  }

  const fn feature(self) -> &'static str {
    match self {
      Self::L2 => "l2_distance",
      Self::Cosine => "cosine_distance",
      Self::NegInnerProduct => "neg_inner_product",
    }
  }
}

/// A rendered pgvector distance expression for a select list or `ORDER BY`.
///
/// ASC is nearest for all three operators (pgvector’s convention for `<#>`).
/// The query vector is embedded as a SQL literal — see
/// [`super::literal::vector_literal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgVectorDistance {
  sql: String,
}

impl PgVectorDistance {
  pub(super) fn new(sql: String) -> Self {
    Self { sql }
  }

  /// The expression as SQL text — what `SelectBuilder::column_expr` takes.
  #[must_use]
  pub fn sql(&self) -> &str {
    &self.sql
  }

  /// `ORDER BY <expr> ASC` — nearest first.
  #[must_use]
  pub fn asc(&self) -> OrderBy {
    OrderBy::raw_asc(&self.sql)
  }

  /// `ORDER BY <expr> DESC` — farthest first.
  #[must_use]
  pub fn desc(&self) -> OrderBy {
    OrderBy::raw_desc(&self.sql)
  }
}

impl From<PgVectorDistance> for OrderBy {
  fn from(distance: PgVectorDistance) -> Self {
    distance.asc()
  }
}

/// `"col" <-> '[…]'::vector` for an explicit dialect.
///
/// # Errors
///
/// - [`DbCoreError::PgVectorUnsupportedDialect`] for [`Dialect::Sqlite`]
/// - [`DbCoreError::PgVectorInvalidArgument`] when an embedding element is not finite
pub fn l2_distance_for(
  dialect: Dialect,
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  distance_for(dialect, column, DistanceOp::L2, query)
}

/// [`l2_distance_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`l2_distance_for`].
pub fn l2_distance(
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  l2_distance_for(Dialect::CURRENT, column, query)
}

/// Like [`l2_distance_for`] but with `<=>`.
///
/// # Errors
///
/// See [`l2_distance_for`].
pub fn cosine_distance_for(
  dialect: Dialect,
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  distance_for(dialect, column, DistanceOp::Cosine, query)
}

/// [`cosine_distance_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`cosine_distance_for`].
pub fn cosine_distance(
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  cosine_distance_for(Dialect::CURRENT, column, query)
}

/// Like [`l2_distance_for`] but with `<#>`.
///
/// # Errors
///
/// See [`l2_distance_for`].
pub fn neg_inner_product_for(
  dialect: Dialect,
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  distance_for(dialect, column, DistanceOp::NegInnerProduct, query)
}

/// [`neg_inner_product_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`neg_inner_product_for`].
pub fn neg_inner_product(
  column: &Column<Vector>,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  neg_inner_product_for(Dialect::CURRENT, column, query)
}

pub(super) fn distance_for(
  dialect: Dialect,
  column: &Column<Vector>,
  op: DistanceOp,
  query: &[f32],
) -> Result<PgVectorDistance, DbCoreError> {
  let feature = op.feature();
  require_postgres(feature, dialect)?;
  let lit = vector_literal(feature, query)?;
  // `Column::qualified` already emits `"table"."column"`; identifiers come from
  // `&'static str` macros / constructors, same as `pg_fts` document SQL.
  Ok(PgVectorDistance::new(format!(
    "{} {} {lit}",
    column.qualified(),
    op.sql_op()
  )))
}
