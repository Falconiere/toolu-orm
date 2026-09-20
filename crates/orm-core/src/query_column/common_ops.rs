//! Operations every column type supports, whatever its SQL type.

use crate::expr::Expr;
use crate::value::Value;

use super::bind::tag_column_bind;
use super::column::Column;

pub trait CommonOps {
  fn eq<V: Into<Value>>(&self, val: V) -> Expr;
  fn ne<V: Into<Value>>(&self, val: V) -> Expr;
  fn in_list(&self, values: &[Value]) -> Expr;
  fn not_in(&self, values: &[Value]) -> Expr;
  fn is_null(&self) -> Expr;
  fn is_not_null(&self) -> Expr;
}

impl<T: 'static> CommonOps for Column<T> {
  fn eq<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "=", tag_column_bind::<T>(val.into()))
  }

  fn ne<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "!=", tag_column_bind::<T>(val.into()))
  }

  fn in_list(&self, values: &[Value]) -> Expr {
    Expr::in_list(self.qualified(), tag_all::<T>(values), false)
  }

  fn not_in(&self, values: &[Value]) -> Expr {
    Expr::in_list(self.qualified(), tag_all::<T>(values), true)
  }

  fn is_null(&self) -> Expr {
    Expr::is_null(self.qualified(), false)
  }

  fn is_not_null(&self) -> Expr {
    Expr::is_null(self.qualified(), true)
  }
}

fn tag_all<T: 'static>(values: &[Value]) -> Vec<Value> {
  values.iter().cloned().map(tag_column_bind::<T>).collect()
}
