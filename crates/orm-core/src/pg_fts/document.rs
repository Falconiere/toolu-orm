//! The left-hand side of `@@` / `ts_rank`: a `tsvector` column or `to_tsvector`.

use crate::column::Text;
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::query_column::Column;

use super::literal::{quoted_config, require_postgres};

const COLUMN: &str = "tsvector column";
const TO_TSVECTOR: &str = "to_tsvector";

/// A rendered `tsvector` expression for the left side of `@@` or `ts_rank`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgTsDocument {
  pub(super) sql: String,
}

impl PgTsDocument {
  /// The document as SQL text.
  #[must_use]
  pub fn sql(&self) -> &str {
    &self.sql
  }
}

/// A precomputed `tsvector` column (`"docs"."search_vector"`) for an explicit
/// dialect.
///
/// There is no `TsVector` marker type — callers name the column as
/// [`Column<Text>`]. Postgres accepts the qualified name as a tsvector
/// expression at execute time.
///
/// # Errors
///
/// [`DbCoreError::PgFtsUnsupportedDialect`] for [`Dialect::Sqlite`].
pub fn column_for(dialect: Dialect, col: &Column<Text>) -> Result<PgTsDocument, DbCoreError> {
  require_postgres(COLUMN, dialect)?;
  Ok(PgTsDocument {
    sql: col.qualified(),
  })
}

/// [`column_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`column_for`].
pub fn column(col: &Column<Text>) -> Result<PgTsDocument, DbCoreError> {
  column_for(Dialect::CURRENT, col)
}

/// `to_tsvector('<config>', "<table>"."<column>")` for an explicit dialect.
///
/// # Errors
///
/// - [`DbCoreError::PgFtsUnsupportedDialect`] for [`Dialect::Sqlite`].
/// - [`DbCoreError::PgFtsInvalidArgument`] when `config` is not a plain
///   identifier.
pub fn to_tsvector_for(
  dialect: Dialect,
  config: &str,
  col: &Column<Text>,
) -> Result<PgTsDocument, DbCoreError> {
  require_postgres(TO_TSVECTOR, dialect)?;
  let config = quoted_config(TO_TSVECTOR, config)?;
  Ok(PgTsDocument {
    sql: format!("{TO_TSVECTOR}({config}, {})", col.qualified()),
  })
}

/// [`to_tsvector_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`to_tsvector_for`].
pub fn to_tsvector(config: &str, col: &Column<Text>) -> Result<PgTsDocument, DbCoreError> {
  to_tsvector_for(Dialect::CURRENT, config, col)
}
