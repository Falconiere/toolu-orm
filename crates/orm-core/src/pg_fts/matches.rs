//! `document @@ {to,plainto,websearch_to}_tsquery(config, $N)`.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::Expr;
use crate::value::Value;

use super::document::PgTsDocument;
use super::literal::{quoted_config, require_postgres};

const TO_TSQUERY: &str = "to_tsquery";
const PLAINTO_TSQUERY: &str = "plainto_tsquery";
const WEBSEARCH_TO_TSQUERY: &str = "websearch_to_tsquery";

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
    self.matches_query_fn(dialect, TO_TSQUERY, config, query.into())
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
    self.matches_query_fn(dialect, PLAINTO_TSQUERY, config, query.into())
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
    self.matches_query_fn(dialect, WEBSEARCH_TO_TSQUERY, config, query.into())
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
    query_fn: &'static str,
    config: &str,
    pattern: Value,
  ) -> Result<Expr, DbCoreError> {
    require_postgres(query_fn, dialect)?;
    let config = quoted_config(query_fn, config)?;
    Ok(Expr::ts_match_target(
      self.sql.clone(),
      query_fn,
      config,
      pattern,
    ))
  }
}
