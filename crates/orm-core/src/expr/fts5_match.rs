//! The FTS5 `MATCH` operator as an [`Expr`], so a full-text predicate sits in
//! `filter(...)` next to the ordinary ones instead of forcing the whole query
//! into hand-written SQL.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::fts5::literal::{quoted_table, require_sqlite};
use crate::value::Value;

use super::types::{Expr, ExprKind};

const TABLE_MATCH: &str = "MATCH";

impl Expr {
  /// `<table> MATCH ?` for an explicit dialect — the whole-table form, which
  /// searches every indexed column and is the common case.
  ///
  /// The pattern is bound, not interpolated, and stays an opaque FTS5 query
  /// string: `NEAR`, `OR`, prefix `*` and column filters are the caller's to
  /// write, and a malformed one comes back as a driver error.
  ///
  /// ```
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::expr::Expr;
  ///
  /// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
  /// let hit = Expr::table_match_for(Dialect::Sqlite, "memory_fts", "runner")?;
  /// let (sql, params) = hit.to_sql_fragment_for(1, Dialect::Sqlite);
  /// assert_eq!(sql, r#""memory_fts" MATCH ?1"#);
  /// assert_eq!(params.len(), 1);
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// - [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`]:
  ///   `MATCH` is SQLite's, and Postgres full-text is a different model.
  ///   Rejecting at construction means no Postgres statement can contain one.
  /// - [`DbCoreError::Fts5InvalidArgument`] when `table` is not a plain
  ///   identifier. FTS5 does not accept a table alias here either.
  ///
  /// Rendering an expression built for SQLite with
  /// [`Expr::to_sql_fragment_for(_, Dialect::Postgres)`](Expr::to_sql_fragment_for)
  /// emits its SQLite text; that takes two contradictory dialect arguments in
  /// a row and is the same latitude [`Expr::raw`] has.
  pub fn table_match_for(
    dialect: Dialect,
    table: &str,
    pattern: impl Into<Value>,
  ) -> Result<Self, DbCoreError> {
    require_sqlite(TABLE_MATCH, dialect)?;
    let target = quoted_table(TABLE_MATCH, table)?;
    Ok(Self::match_target(target, pattern.into()))
  }

  /// [`Expr::table_match_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Expr::table_match_for`].
  pub fn table_match(table: &str, pattern: impl Into<Value>) -> Result<Self, DbCoreError> {
    Self::table_match_for(Dialect::CURRENT, table, pattern)
  }

  pub(crate) fn match_target(target: String, pattern: Value) -> Self {
    Self {
      kind: ExprKind::Match { target, pattern },
    }
  }
}
