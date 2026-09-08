//! The synthesised `distance` column `vec0` adds to every KNN row.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::OrderBy;

use super::require::require_sqlite;

const FEATURE: &str = "distance";
const SQL: &str = "\"distance\"";

/// The `distance` output column sqlite-vec synthesises for a KNN row.
///
/// It is not in the table's declared columns. Use [`Self::sql`] with
/// `SelectBuilder::column_expr` (or include `"distance"` in `columns_raw`),
/// and [`Self::asc`] (or `Into<OrderBy>`) for `ORDER BY` — ascending is
/// nearest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vec0Distance;

impl Vec0Distance {
  /// The identifier as SQL text — what `column_expr` takes for a named output.
  #[must_use]
  pub fn sql(&self) -> &'static str {
    SQL
  }

  /// `ORDER BY "distance" ASC` — nearest first.
  #[must_use]
  pub fn asc(&self) -> OrderBy {
    OrderBy::raw_asc(SQL)
  }

  /// `ORDER BY "distance" DESC` — farthest first.
  #[must_use]
  pub fn desc(&self) -> OrderBy {
    OrderBy::raw_desc(SQL)
  }
}

impl From<Vec0Distance> for OrderBy {
  fn from(distance: Vec0Distance) -> Self {
    distance.asc()
  }
}

/// [`distance`] for an explicit dialect.
///
/// Fallible for the same reason as [`crate::fts5::bm25_for`]: a Postgres-bound
/// value must not exist, so no statement can carry a `vec0` `distance` on that
/// dialect. On a SQLite build the short form [`distance`] still returns
/// `Result` so both lanes share one signature.
///
/// # Errors
///
/// [`DbCoreError::Vec0UnsupportedDialect`] for [`Dialect::Postgres`].
pub fn distance_for(dialect: Dialect) -> Result<Vec0Distance, DbCoreError> {
  require_sqlite(FEATURE, dialect)?;
  Ok(Vec0Distance)
}

/// [`distance_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`distance_for`].
pub fn distance() -> Result<Vec0Distance, DbCoreError> {
  distance_for(Dialect::CURRENT)
}
