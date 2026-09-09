//! `document @@ {to,plainto,websearch_to}_tsquery(config, $N)`.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::Expr;
use crate::value::Value;

use super::document::PgTsDocument;
use super::literal::{quoted_config, require_postgres, TsQueryFn};

impl PgTsDocument {
  /// `document @@ to_tsquery('<config>', $N)` for an explicit dialect.
  ///
  /// The query text is bound, not interpolated. Malformed `to_tsquery` syntax
  /// reaches the server as a driver error.
  ///
  /// # Errors
  ///
  /// - [`DbCoreError::PgFtsUnsupportedDialect`] for [`Dialect::Sqlite`].
  /// - [`DbCoreError::PgFtsInvalidArgument`] when `config` is not a plain
  ///   identifier.
  pub fn matches_tsquery_for(
    &self,
    dialect: Dialect,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_query_fn(dialect, TsQueryFn::ToTsQuery, config, query.into())
  }

  /// [`Self::matches_tsquery_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Self::matches_tsquery_for`].
  pub fn matches_tsquery(
    &self,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_tsquery_for(Dialect::CURRENT, config, query)
  }

  /// `document @@ plainto_tsquery('<config>', $N)` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// See [`Self::matches_tsquery_for`].
  pub fn matches_plainto_tsquery_for(
    &self,
    dialect: Dialect,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_query_fn(dialect, TsQueryFn::PlainToTsQuery, config, query.into())
  }

  /// [`Self::matches_plainto_tsquery_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Self::matches_plainto_tsquery_for`].
  pub fn matches_plainto_tsquery(
    &self,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_plainto_tsquery_for(Dialect::CURRENT, config, query)
  }

  /// `document @@ websearch_to_tsquery('<config>', $N)` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// See [`Self::matches_tsquery_for`].
  pub fn matches_websearch_to_tsquery_for(
    &self,
    dialect: Dialect,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_query_fn(dialect, TsQueryFn::WebsearchToTsQuery, config, query.into())
  }

  /// [`Self::matches_websearch_to_tsquery_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Self::matches_websearch_to_tsquery_for`].
  pub fn matches_websearch_to_tsquery(
    &self,
    config: &str,
    query: impl Into<Value>,
  ) -> Result<Expr, DbCoreError> {
    self.matches_websearch_to_tsquery_for(Dialect::CURRENT, config, query)
  }

  fn matches_query_fn(
    &self,
    dialect: Dialect,
    query_fn: TsQueryFn,
    config: &str,
    pattern: Value,
  ) -> Result<Expr, DbCoreError> {
    let name = query_fn.as_sql();
    require_postgres(name, dialect)?;
    let config = quoted_config(name, config)?;
    Ok(Expr::ts_match_target(
      self.sql.clone(),
      name,
      config,
      pattern,
    ))
  }
}
