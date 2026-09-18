//! The predicates that hold a whole statement: `EXISTS` and `NOT EXISTS`.

use crate::expr::SelectSource;

use super::predicate::{Expr, ExprKind};

impl Expr {
  /// `EXISTS (<query>)` — true when the subquery returns at least one row.
  ///
  /// Correlate it by naming an outer column inside `query`; the inner builder
  /// renders `"outer"."column"` like any other qualified reference, so no
  /// extra machinery is involved.
  pub fn exists(query: impl SelectSource + 'static) -> Self {
    Self::exists_node(query, false)
  }

  /// `NOT EXISTS (<query>)` — the NULL-safe complement of an `IN` test.
  ///
  /// A correlated `NOT EXISTS` keeps a row whose key is NULL, because no inner
  /// row matches it. `NOT IN` over a set that contains a NULL returns *nothing*
  /// at all, on SQLite and Postgres alike: three-valued logic, not a builder
  /// quirk. Prefer this form whenever either side may be NULL.
  pub fn not_exists(query: impl SelectSource + 'static) -> Self {
    Self::exists_node(query, true)
  }

  fn exists_node(query: impl SelectSource + 'static, negated: bool) -> Self {
    Self {
      kind: ExprKind::Exists {
        query: Box::new(query),
        negated,
      },
    }
  }
}
