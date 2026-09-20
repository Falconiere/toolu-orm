//! Scalars built from a whole statement: `IN (…)`, `NOT IN (…)`, `(SELECT …)`.

use crate::expr::types::ExprKind;
use crate::expr::{Expr, SelectSource};

use super::types::{Scalar, ScalarKind};

impl Scalar {
  /// `<self> IN (<query>)` — membership tested database-side.
  ///
  /// The id set never travels through Rust, which is what makes a set-based
  /// `DELETE … WHERE id IN (SELECT …)` one statement instead of a read
  /// followed by a write.
  #[must_use]
  pub fn in_subquery(self, query: impl SelectSource + 'static) -> Expr {
    Self::in_subquery_node(self, query, false)
  }

  /// `<self> NOT IN (<query>)`.
  ///
  /// Returns **no** rows when the subquery's set contains a NULL, on both
  /// dialects; see [`Expr::not_exists`] for the NULL-safe form.
  #[must_use]
  pub fn not_in_subquery(self, query: impl SelectSource + 'static) -> Expr {
    Self::in_subquery_node(self, query, true)
  }

  /// `(SELECT …)` in value position — a projection, an `ORDER BY` term, or one
  /// side of a comparison.
  ///
  /// Scalar SQL requires exactly one selected column; the builder does not
  /// validate that shape. Postgres rejects multiple rows; SQLite takes the
  /// first. Both return NULL for an empty result.
  #[must_use]
  pub fn subquery(query: impl SelectSource + 'static) -> Self {
    Self::from_kind(ScalarKind::Subquery(Box::new(query)))
  }

  fn in_subquery_node(left: Scalar, query: impl SelectSource + 'static, negated: bool) -> Expr {
    Expr::from_kind(ExprKind::InSubquery {
      left,
      query: Box::new(query),
      negated,
    })
  }
}
