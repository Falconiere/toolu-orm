//! Full-text `MATCH` narrowed to one column of an FTS5 table.

use crate::column::Text;
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::Expr;
use crate::fts5::literal::require_sqlite;
use crate::value::Value;

use super::column::Column;

/// Full-text `MATCH` restricted to one column of an FTS5 table.
///
/// The whole-table form — [`Expr::table_match`], which searches every indexed
/// column — is the common case; this narrows the search to this column.
pub trait Fts5Ops {
  /// `"<table>"."<column>" MATCH ?` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`]; see
  /// [`Expr::table_match_for`].
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, pattern: V) -> Result<Expr, DbCoreError>;

  /// [`Fts5Ops::matches_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Fts5Ops::matches_for`].
  fn matches<V: Into<Value>>(&self, pattern: V) -> Result<Expr, DbCoreError> {
    self.matches_for(Dialect::CURRENT, pattern)
  }
}

/// FTS5 stores every column as text and `#[fts5_table]` declares them `Text`,
/// so this is the only column type a `MATCH` can name.
impl Fts5Ops for Column<Text> {
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, pattern: V) -> Result<Expr, DbCoreError> {
    require_sqlite("MATCH", dialect)?;
    Ok(Expr::match_target(self.qualified(), pattern.into()))
  }
}
