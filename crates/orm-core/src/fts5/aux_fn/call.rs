//! [`Fts5Fn`] — a rendered FTS5 auxiliary-function call.

use crate::expr::OrderBy;

/// A rendered FTS5 auxiliary-function call, ready to drop into a select list
/// or an `ORDER BY`.
///
/// FTS5 auxiliary functions bind no parameters: the weights, the tags and the
/// column index are all literals in the SQL text (the engine rejects `?` in
/// those positions). So this is plain SQL, not an [`crate::expr::Expr`] node —
/// there is nothing to bind and nothing to renumber.
///
/// ```
/// use toolu_orm_core::fts5;
///
/// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
/// let score = fts5::bm25("memory_fts", &[0.0, 1.0, 3.0])?;
/// assert_eq!(score.sql(), r#"bm25("memory_fts", 0.0, 1.0, 3.0)"#);
/// assert_eq!(score.asc().to_sql(), r#"bm25("memory_fts", 0.0, 1.0, 3.0) ASC"#);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fts5Fn {
  sql: String,
}

impl Fts5Fn {
  pub(super) fn new(sql: String) -> Self {
    Self { sql }
  }

  /// The call as SQL text — what `SelectBuilder::column_expr` takes to make it
  /// a named output.
  #[must_use]
  pub fn sql(&self) -> &str {
    &self.sql
  }

  /// `ORDER BY <call> ASC`.
  ///
  /// For [`crate::fts5::bm25`] and [`crate::fts5::rank`] this is **best
  /// first**: FTS5 relevance scores are negative and a better match is more
  /// negative. Ordering a score `DESC` returns the worst matches first, which
  /// is the mistake everyone makes once.
  #[must_use]
  pub fn asc(&self) -> OrderBy {
    OrderBy::raw_asc(&self.sql)
  }

  /// `ORDER BY <call> DESC` — worst match first for a relevance score; see
  /// [`Fts5Fn::asc`].
  #[must_use]
  pub fn desc(&self) -> OrderBy {
    OrderBy::raw_desc(&self.sql)
  }
}
