//! Ordering comparisons on the column types that hold numbers and instants.

use crate::column::{BigInt, Date, Integer, Real, SmallInt, Time, Timestamp};
use crate::expr::Expr;
use crate::value::Value;

use super::bind::tag_column_bind;
use super::column::Column;

pub trait NumericOps {
  fn gt<V: Into<Value>>(&self, val: V) -> Expr;
  fn lt<V: Into<Value>>(&self, val: V) -> Expr;
  fn gte<V: Into<Value>>(&self, val: V) -> Expr;
  fn lte<V: Into<Value>>(&self, val: V) -> Expr;
  fn between<L: Into<Value>, H: Into<Value>>(&self, low: L, high: H) -> Expr;
}

macro_rules! impl_numeric_ops {
  ($($ty:ty),+) => {
    $(
      impl NumericOps for Column<$ty> {
        fn gt<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), ">", tag_column_bind::<$ty>(val.into()))
        }

        fn lt<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "<", tag_column_bind::<$ty>(val.into()))
        }

        fn gte<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), ">=", tag_column_bind::<$ty>(val.into()))
        }

        fn lte<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "<=", tag_column_bind::<$ty>(val.into()))
        }

        fn between<L: Into<Value>, H: Into<Value>>(&self, low: L, high: H) -> Expr {
          Expr::between(
            self.qualified(),
            tag_column_bind::<$ty>(low.into()),
            tag_column_bind::<$ty>(high.into()),
          )
        }
      }
    )+
  };
}

impl_numeric_ops!(Integer, Real, BigInt, SmallInt, Timestamp, Date, Time);
