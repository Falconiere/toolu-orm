//! Operations every column type supports, whatever its SQL type.

use crate::expr::Expr;
use crate::value::Value;

use super::column::Column;

pub trait CommonOps {
  fn eq<V: Into<Value>>(&self, val: V) -> Expr;
  fn ne<V: Into<Value>>(&self, val: V) -> Expr;
  fn in_list(&self, values: &[Value]) -> Expr;
  fn not_in(&self, values: &[Value]) -> Expr;
  fn is_null(&self) -> Expr;
  fn is_not_null(&self) -> Expr;
}

impl<T> CommonOps for Column<T> {
  fn eq<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "=", val.into())
  }

  fn ne<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "!=", val.into())
  }

  fn in_list(&self, values: &[Value]) -> Expr {
    Expr::in_list(self.qualified(), values.to_vec(), false)
  }

  fn not_in(&self, values: &[Value]) -> Expr {
    Expr::in_list(self.qualified(), values.to_vec(), true)
  }

  fn is_null(&self) -> Expr {
    Expr::is_null(self.qualified(), false)
  }

  fn is_not_null(&self) -> Expr {
    Expr::is_null(self.qualified(), true)
  }
}
