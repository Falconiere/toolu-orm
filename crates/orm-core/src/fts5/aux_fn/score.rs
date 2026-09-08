//! The FTS5 relevance functions: `bm25()` and the `rank` column.

use crate::dialect::Dialect;
use crate::error::DbCoreError;

use super::super::literal::{float_literal, quoted_table, require_sqlite};
use super::call::Fts5Fn;

const BM25: &str = "bm25";
const RANK: &str = "rank";

/// `bm25(<table>[, <weight>…])` for an explicit dialect.
///
/// One weight per column of the FTS5 table, in declaration order, including
/// any `UNINDEXED` column. An empty slice renders `bm25("t")`, FTS5's
/// documented default of 1.0 for every column; `0.0` excludes a column from
/// the score.
///
/// **The score is negative, and a better match is more negative**, so
/// `ORDER BY score ASC` is best first.
///
/// Weights are formatted into the SQL text because FTS5 rejects bound
/// parameters in this position. That is safe only because this validates them
/// first, which is exactly the guarantee a `format!` at the call site does not
/// give.
///
/// # Errors
///
/// - [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`].
/// - [`DbCoreError::Fts5InvalidArgument`] when `table` is not a plain
///   identifier, or a weight is negative or not finite. A negative weight is
///   refused because it makes the score *positive*, silently inverting the
///   ordering every caller of this function depends on.
pub fn bm25_for(dialect: Dialect, table: &str, weights: &[f64]) -> Result<Fts5Fn, DbCoreError> {
  require_sqlite(BM25, dialect)?;
  let table = quoted_table(BM25, table)?;

  let mut sql = format!("{BM25}({table}");
  for (index, weight) in weights.iter().enumerate() {
    validate_weight(index, *weight)?;
    sql.push_str(", ");
    sql.push_str(&float_literal(*weight));
  }
  sql.push(')');
  Ok(Fts5Fn::new(sql))
}

/// [`bm25_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`bm25_for`].
pub fn bm25(table: &str, weights: &[f64]) -> Result<Fts5Fn, DbCoreError> {
  bm25_for(Dialect::CURRENT, table, weights)
}

/// The FTS5 `rank` column for an explicit dialect.
///
/// `rank` is a hidden column, not a function, so it renders as the bare word —
/// `rank(t)` is not valid SQL. It is `bm25()` with default weights, and it is
/// negative in the same way, so `ORDER BY rank ASC` is best first.
///
/// `table` is still taken and validated so the call reads like its siblings
/// and mis-spelling the table is caught in the same place.
///
/// # Errors
///
/// - [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`].
/// - [`DbCoreError::Fts5InvalidArgument`] when `table` is not a plain identifier.
pub fn rank_for(dialect: Dialect, table: &str) -> Result<Fts5Fn, DbCoreError> {
  require_sqlite(RANK, dialect)?;
  quoted_table(RANK, table)?;
  Ok(Fts5Fn::new(RANK.to_owned()))
}

/// [`rank_for`] against [`Dialect::CURRENT`].
///
/// # Errors
///
/// See [`rank_for`].
pub fn rank(table: &str) -> Result<Fts5Fn, DbCoreError> {
  rank_for(Dialect::CURRENT, table)
}

fn validate_weight(index: usize, weight: f64) -> Result<(), DbCoreError> {
  let reason = if weight.is_nan() {
    "is NaN"
  } else if weight.is_infinite() {
    "is infinite"
  } else if weight < 0.0 {
    "is negative, which makes the bm25 score positive and inverts the ranking"
  } else {
    return Ok(());
  };
  Err(DbCoreError::Fts5InvalidArgument {
    function: BM25.to_owned(),
    reason: format!("weight #{index} ({weight}) {reason}; weights must be finite and >= 0"),
  })
}
