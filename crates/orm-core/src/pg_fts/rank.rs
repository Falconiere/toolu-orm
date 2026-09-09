//! `ts_rank` as a selectable / orderable SQL call.
//!
//! The query text is dollar-quoted into the SQL because
//! `SelectBuilder::column_expr` and [`OrderBy`] carry SQL text only — they
//! cannot bind parameters. Prefer binding the same string through
//! [`super::PgTsDocument::matches_tsquery_for`] in `WHERE`.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::OrderBy;

use super::document::PgTsDocument;
use super::literal::{dollar_quote, quoted_config, require_postgres, weights_literal, TsQueryFn};

const TS_RANK: &str = "ts_rank";

/// A rendered `ts_rank(...)` call, ready for a select list or `ORDER BY`.
///
/// **The score is positive, and a better match is higher**, so
/// `ORDER BY score DESC` is best first — the inverse of FTS5 `bm25`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgFtsFn {
  sql: String,
}

impl PgFtsFn {
  fn new(sql: String) -> Self {
    Self { sql }
  }

  /// The call as SQL text — what `SelectBuilder::column_expr` takes.
  #[must_use]
  pub fn sql(&self) -> &str {
    &self.sql
  }

  /// `ORDER BY <call> ASC` — worst match first for a relevance score.
  #[must_use]
  pub fn asc(&self) -> OrderBy {
    OrderBy::raw_asc(&self.sql)
  }

  /// `ORDER BY <call> DESC` — **best first** for a relevance score.
  #[must_use]
  pub fn desc(&self) -> OrderBy {
    OrderBy::raw_desc(&self.sql)
  }
}

/// `ts_rank([weights,] document, to_tsquery(config, $q$query$q$))` for an
/// explicit dialect.
///
/// # Errors
///
/// - [`DbCoreError::PgFtsUnsupportedDialect`] for [`Dialect::Sqlite`].
/// - [`DbCoreError::PgFtsInvalidArgument`] when `config` is invalid or a weight
///   is not finite and non-negative.
pub fn ts_rank_tsquery_for(
  dialect: Dialect,
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  rank_for(dialect, document, TsQueryFn::To, config, query, weights)
}

/// [`ts_rank_tsquery_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`ts_rank_tsquery_for`].
pub fn ts_rank_tsquery(
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  ts_rank_tsquery_for(Dialect::CURRENT, document, config, query, weights)
}

/// Like [`ts_rank_tsquery_for`] but with `plainto_tsquery`.
///
/// # Errors
///
/// See [`ts_rank_tsquery_for`].
pub fn ts_rank_plainto_tsquery_for(
  dialect: Dialect,
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  rank_for(dialect, document, TsQueryFn::Plain, config, query, weights)
}

/// [`ts_rank_plainto_tsquery_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`ts_rank_plainto_tsquery_for`].
pub fn ts_rank_plainto_tsquery(
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  ts_rank_plainto_tsquery_for(Dialect::CURRENT, document, config, query, weights)
}

/// Like [`ts_rank_tsquery_for`] but with `websearch_to_tsquery`.
///
/// # Errors
///
/// See [`ts_rank_tsquery_for`].
pub fn ts_rank_websearch_to_tsquery_for(
  dialect: Dialect,
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  rank_for(
    dialect,
    document,
    TsQueryFn::Websearch,
    config,
    query,
    weights,
  )
}

/// [`ts_rank_websearch_to_tsquery_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`ts_rank_websearch_to_tsquery_for`].
pub fn ts_rank_websearch_to_tsquery(
  document: &PgTsDocument,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  ts_rank_websearch_to_tsquery_for(Dialect::CURRENT, document, config, query, weights)
}

fn rank_for(
  dialect: Dialect,
  document: &PgTsDocument,
  query_fn: TsQueryFn,
  config: &str,
  query: &str,
  weights: Option<&[f32; 4]>,
) -> Result<PgFtsFn, DbCoreError> {
  require_postgres(TS_RANK, dialect)?;
  let config = quoted_config(TS_RANK, config)?;
  let query_fn_sql = query_fn.as_sql();
  let query_lit = dollar_quote(query);
  let query_call = format!("{query_fn_sql}({config}, {query_lit})");
  let doc_sql = document.sql();
  let sql = match weights {
    Some(weights) => {
      let w = weights_literal(TS_RANK, weights)?;
      format!("{TS_RANK}({w}, {doc_sql}, {query_call})")
    },
    None => format!("{TS_RANK}({doc_sql}, {query_call})"),
  };
  Ok(PgFtsFn::new(sql))
}
