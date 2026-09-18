//! One `ORDER BY` term.

use crate::dialect::Dialect;
use crate::expr::{BoundParams, Scalar};
use crate::value::Value;

/// One `ORDER BY` term: a scalar and a direction.
///
/// Build one with `Column::asc` / `Column::desc`, [`Scalar::asc`] /
/// [`Scalar::desc`] for a computed term, or [`OrderBy::alias_asc`] to name a
/// projection's alias.
pub struct OrderBy {
  pub(crate) term: Scalar,
  pub(crate) direction: &'static str,
}

impl OrderBy {
  /// Renders into `params`, sharing its binding ledger; see
  /// [`Expr::render_into`](crate::expr::Expr::render_into).
  #[must_use]
  pub fn render_into(&self, params: &mut BoundParams, dialect: Dialect) -> String {
    let sql = self.term.render_into(params, dialect);
    format!("{sql} {}", self.direction)
  }

  /// The term and its direction, with placeholders numbered from `start` and
  /// the values the term binds returned alongside.
  #[must_use]
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params = BoundParams::new();
    let sql = params.nested(start, |nested| self.render_into(nested, dialect));
    (sql, params.into_values())
  }

  /// The parameter-free rendering.
  ///
  /// Every term built from a column, an alias or pre-rendered SQL binds
  /// nothing, which is what this is for. A term that *does* bind — a `CASE`,
  /// or a call over [`Scalar::bind`] — needs
  /// [`OrderBy::to_sql_fragment_for`], which is what the query builders use;
  /// this method would number its placeholders from 1 and drop the values.
  #[must_use]
  pub fn to_sql(&self) -> String {
    self.to_sql_fragment_for(1, Dialect::CURRENT).0
  }

  /// `ORDER BY "<alias>" ASC` — order by a computed output rather than
  /// repeating the expression that produced it.
  ///
  /// Pair it with `SelectBuilder::column_expr(expr, alias)`. For an FTS5
  /// relevance score `ASC` is best first, because `bm25()` is negative.
  #[must_use]
  pub fn alias_asc(alias: &str) -> Self {
    Self::raw_asc(&format!("\"{alias}\""))
  }

  /// `ORDER BY "<alias>" DESC`; see [`OrderBy::alias_asc`].
  #[must_use]
  pub fn alias_desc(alias: &str) -> Self {
    Self::raw_desc(&format!("\"{alias}\""))
  }

  pub(crate) fn from_term(term: Scalar, direction: &'static str) -> Self {
    Self { term, direction }
  }

  pub(crate) fn raw_asc(sql: &str) -> Self {
    Self::from_term(Scalar::sql(sql), "ASC")
  }

  pub(crate) fn raw_desc(sql: &str) -> Self {
    Self::from_term(Scalar::sql(sql), "DESC")
  }
}
