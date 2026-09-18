//! [`SelectBuilder`] as a statement another statement can hold.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::SelectSource;
use toolu_orm_core::value::Value;

use super::SelectBuilder;

/// Lets a whole `SELECT` sit inside `IN (…)`, `EXISTS (…)`, a scalar
/// projection, or — for issue #114 — after `INSERT INTO "t" (…) `.
impl SelectSource for SelectBuilder {
  /// The statement rendered so that its *first* placeholder is `start`.
  ///
  /// Every clause helper numbers from `params.len() + 1`, which hardcodes the
  /// base at 1. So this opens an **offset frame**: pre-fill the vector with
  /// the `start - 1` values the caller already emitted, render into it, split
  /// the tail back off. Identical by construction — the helpers read
  /// `params.len()` and never the contents — and the filler never reaches a
  /// statement.
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let emitted = start.saturating_sub(1);
    let mut sql = String::new();
    let mut params: Vec<Value> = vec![Value::Null; emitted];

    self.push_statement(&mut sql, &mut params, dialect, self.limit_val);

    (sql, params.split_off(emitted))
  }
}
