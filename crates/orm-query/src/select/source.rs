//! [`SelectBuilder`] as a statement another statement can hold.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{BoundParams, SelectSource};
use toolu_orm_core::value::Value;

use super::SelectBuilder;

impl SelectBuilder {
  /// The statement rendered so that its *first* placeholder is `start`,
  /// appending only what it binds to `params`.
  ///
  /// Every clause helper numbers from `BoundParams::next_index`, which counts
  /// from the buffer's start, so this renders inside [`BoundParams::nested`]:
  /// the frame stands in for the `start - 1` values already emitted, renders,
  /// and drops the stand-ins. The helpers read the length and never the
  /// contents, so `start = N` and `N - 1` entries already present are the same
  /// computation.
  ///
  /// Because `nested` carries the binding ledger both ways, a handle first
  /// used *inside* this statement takes an index the outer statement can
  /// reuse, and one the outer statement already bound is reused in here.
  ///
  /// Both trait methods below call this, rather than one calling the other, so
  /// neither can recurse into the other's default.
  fn push_nested(&self, start: usize, params: &mut BoundParams, dialect: Dialect) -> String {
    params.nested(start, |nested| {
      let mut sql = String::new();
      self.push_statement(&mut sql, nested, dialect, self.limit_val);
      sql
    })
  }
}

/// Lets a whole `SELECT` sit inside `IN (…)`, `EXISTS (…)`, a scalar
/// projection, or — for issue #114 — after `INSERT INTO "t" (…) `.
impl SelectSource for SelectBuilder {
  /// The statement and the values it binds, first placeholder at `start`.
  ///
  /// A standalone rendering: the ledger is created here and dropped here, so a
  /// handle used twice *inside* the statement shares one placeholder while one
  /// it shares with the caller binds on each side.
  fn to_select_sql_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let mut params = BoundParams::new();
    let sql = self.push_nested(start, &mut params, dialect);
    (sql, params.into_values())
  }

  /// The statement appended to `params`, sharing the caller's binding ledger.
  ///
  /// `composition_sql_test::subquery` pins the numbering across the boundary;
  /// `reusable_bind_sql_test::nesting::a_handle_first_used_inside_a_subquery_is_reused_by_a_later_clause`
  /// and its `…_first_used_outside_is_reused_inside_a_subquery` twin pin both
  /// directions of the shared binding.
  fn to_select_sql_into(&self, start: usize, params: &mut BoundParams, dialect: Dialect) -> String {
    self.push_nested(start, params, dialect)
  }
}
