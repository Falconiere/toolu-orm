//! [`SelectBuilder`] as a statement another statement can hold.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, SelectSource};
use toolu_orm_core::value::Value;

use super::SelectBuilder;

/// Lets a whole `SELECT` sit inside `IN (…)`, `EXISTS (…)`, a scalar
/// projection, or — for issue #114 — after `INSERT INTO "t" (…) `.
impl SelectSource for SelectBuilder {
  /// The statement rendered so that its *first* placeholder is `start`, with
  /// only the values it binds returned.
  ///
  /// [`Self::to_select_sql_into`] over a fresh buffer: a handle used twice
  /// inside the statement still shares one placeholder, and no ledger is
  /// carried in or out, which is what a standalone rendering means.
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params = BoundParams::new();
    let sql = self.to_select_sql_into(start, &mut params, dialect);
    (sql, params.into_values())
  }

  /// The statement appended to `params`, first placeholder at `start`, sharing
  /// the caller's binding ledger.
  ///
  /// Every clause helper numbers from `params.len() + 1`, which hardcodes the
  /// base at 1, so this renders inside [`BoundParams::nested`]: it stands in
  /// for the `start - 1` values already emitted, renders, and drops the
  /// stand-ins. The helpers read the length and never the contents, so
  /// `start = N` and `N - 1` entries already present are the same computation.
  ///
  /// Because `nested` carries the ledger both ways, a handle first used inside
  /// this statement takes an index the outer statement can reuse, and one the
  /// outer statement already bound is reused in here.
  /// `composition_sql_test::subquery` asserts both halves of that.
  fn to_select_sql_into(&self, start: usize, params: &mut BoundParams, dialect: Dialect) -> String {
    params.nested(start, |nested| {
      let mut sql = String::new();
      self.push_statement(&mut sql, nested, dialect, self.limit_val);
      sql
    })
  }
}
