//! The `LIMIT` / `OFFSET` tail of a SELECT, and the bounded first-row form of
//! the query that [`SelectBuilder::fetch_one`] and
//! [`SelectBuilder::fetch_optional`] send.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;

use super::SelectBuilder;

/// The row bound a first-row fetch asks the database for.
///
/// `min(n, 1)`, with an absent limit reading as "no bound yet". A minimum, not
/// a clamp: a value below `1` is already smaller, so `0` and any negative limit
/// pass through unchanged rather than being pulled up to `1`.
///
/// - no limit — `1`; the caller wants one row, so ask for one row.
/// - a positive limit — `1`; `LIMIT` truncates after `ORDER BY` and `OFFSET`,
///   so the first row of a page of `n` is the first row of a page of one.
/// - `0` — `0`; an explicitly empty page stays empty.
/// - a negative limit — unchanged. SQLite reads it as "no limit" and Postgres
///   rejects it; clamping would change an observable result on one driver or
///   the other, so this one query shape keeps its driver-defined meaning.
fn first_row_limit(limit_val: Option<i64>) -> i64 {
  limit_val.map_or(1, |limit| limit.min(1))
}

impl SelectBuilder {
  /// [`Self::to_sql_for`] bounded to at most one row.
  ///
  /// Identical to `to_sql_for` — same select list, joins, `WHERE` and its
  /// parameters, `ORDER BY`, `OFFSET` — except that the `LIMIT` is always
  /// present and never asks for more than one row. Parameters are pushed as
  /// their placeholders are written, so the numbering stays self-consistent.
  pub fn to_first_row_sql_for(&self, dialect: Dialect) -> (String, Vec<Value>) {
    self.to_sql_with_limit(dialect, Some(first_row_limit(self.limit_val)))
  }

  /// [`Self::to_first_row_sql_for`] against [`Dialect::CURRENT`].
  pub fn to_first_row_sql(&self) -> (String, Vec<Value>) {
    self.to_first_row_sql_for(Dialect::CURRENT)
  }

  /// Appends `LIMIT`/`OFFSET`, binding each as a parameter.
  ///
  /// `limit` is passed in rather than read off the builder so the first-row
  /// form can substitute its own bound. SQLite rejects a bare `OFFSET`, so an
  /// absent `limit` with an offset set renders a literal (unbound) `LIMIT -1`
  /// first — SQLite's "no limit" — leaving Postgres's standalone `OFFSET`
  /// untouched.
  pub(super) fn append_limit_offset_for(
    &self,
    sql: &mut String,
    params: &mut Vec<Value>,
    dialect: Dialect,
    limit: Option<i64>,
  ) {
    if let Some(limit) = limit {
      let idx = params.len() + 1;
      params.push(Value::Integer(limit));
      sql.push_str(&format!(" LIMIT {}", dialect.param(idx)));
    } else if dialect == Dialect::Sqlite && self.offset_val.is_some() {
      sql.push_str(" LIMIT -1");
    }
    if let Some(offset) = self.offset_val {
      let idx = params.len() + 1;
      params.push(Value::Integer(offset));
      sql.push_str(&format!(" OFFSET {}", dialect.param(idx)));
    }
  }
}
