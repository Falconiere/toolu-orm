//! [`JoinCondition`] — the predicate of a `JOIN ... ON` clause.

use crate::dialect::Dialect;
use crate::value::Value;

use super::predicate::Expr;

/// The predicate a `JOIN ... ON` renders: an [`Expr`] in a different coat.
///
/// `ON` and `WHERE` say the same kinds of thing, so the two convert both ways.
/// The wrapper buys the `join` / `left_join` signature and the `and` / `or`
/// that put a column-to-column equality and a bound comparison in one clause:
///
/// ```
/// # use toolu_orm_core::column::{Integer, Text};
/// # use toolu_orm_core::dialect::Dialect;
/// # use toolu_orm_core::query_column::{Column, CommonOps};
/// const F_REPO: Column<Text> = Column::new("feedback", "repo");
/// const C_REPO: Column<Text> = Column::new("symbols", "repo");
/// const F_LIVE: Column<Integer> = Column::new("feedback", "live");
///
/// let (sql, params) = F_REPO.equals(&C_REPO).and(F_LIVE.eq(1))
///   .to_sql_fragment_for(1, Dialect::Sqlite);
/// assert_eq!(sql, r#"("feedback"."repo" = "symbols"."repo" AND "feedback"."live" = ?1)"#);
/// assert_eq!(params.len(), 1);
/// ```
///
/// There is no parameterless `to_sql`: a condition may bind values, and handing
/// back SQL without them would lose the bindings.
pub struct JoinCondition {
  expr: Expr,
}

impl JoinCondition {
  /// Uses `expr` as an `ON` predicate.
  #[must_use]
  pub fn on(expr: Expr) -> Self {
    Self { expr }
  }

  /// `(self AND other)`; `other` may be an [`Expr`] or another condition.
  #[must_use]
  pub fn and(self, other: impl Into<JoinCondition>) -> Self {
    Self {
      expr: self.expr.and(other.into().expr),
    }
  }

  /// `(self OR other)`; `other` may be an [`Expr`] or another condition.
  #[must_use]
  pub fn or(self, other: impl Into<JoinCondition>) -> Self {
    Self {
      expr: self.expr.or(other.into().expr),
    }
  }

  /// SQL fragment with dialect-specific positional parameters starting at
  /// `start`, plus the values they bind, in placeholder order.
  #[must_use]
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    self.expr.to_sql_fragment_for(start, dialect)
  }

  /// [`JoinCondition::to_sql_fragment_for`] against [`Dialect::CURRENT`].
  #[must_use]
  pub fn to_sql_fragment(&self, start: usize) -> (String, Vec<Value>) {
    self.to_sql_fragment_for(start, Dialect::CURRENT)
  }
}

impl From<Expr> for JoinCondition {
  fn from(expr: Expr) -> Self {
    Self { expr }
  }
}

impl From<JoinCondition> for Expr {
  fn from(condition: JoinCondition) -> Self {
    condition.expr
  }
}
