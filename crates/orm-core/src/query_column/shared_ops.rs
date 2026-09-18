//! The reusable-binding twins of the [`CommonOps`](super::CommonOps)
//! predicates.

use crate::alias::QualifiedColumn;
use crate::expr::{Expr, SharedBind, SharedBindList};

/// Equality and `IN` against a handle, so repeated predicates share one
/// placeholder instead of binding the value again.
///
/// Blanket-implemented for every [`QualifiedColumn`], which is both
/// [`Column<T>`](super::Column) and
/// [`AliasedColumn<T>`](crate::alias::AliasedColumn). It is a trait of its own
/// rather than four more methods on `CommonOps` so that an external
/// implementor of `CommonOps` keeps compiling.
pub trait SharedOps {
  /// `<column> = <handle>`.
  fn eq_shared(&self, bind: &SharedBind) -> Expr;

  /// `<column> != <handle>`.
  fn ne_shared(&self, bind: &SharedBind) -> Expr;

  /// `<column> IN (<handle>)`, or `1 = 0` when the handle is empty.
  fn in_shared(&self, list: &SharedBindList) -> Expr;

  /// `<column> NOT IN (<handle>)`, or `1 = 1` when the handle is empty.
  fn not_in_shared(&self, list: &SharedBindList) -> Expr;
}

impl<C: QualifiedColumn + ?Sized> SharedOps for C {
  fn eq_shared(&self, bind: &SharedBind) -> Expr {
    Expr::comparison(self.qualified(), "=", bind)
  }

  fn ne_shared(&self, bind: &SharedBind) -> Expr {
    Expr::comparison(self.qualified(), "!=", bind)
  }

  fn in_shared(&self, list: &SharedBindList) -> Expr {
    Expr::in_list(self.qualified(), list, false)
  }

  fn not_in_shared(&self, list: &SharedBindList) -> Expr {
    Expr::in_list(self.qualified(), list, true)
  }
}
