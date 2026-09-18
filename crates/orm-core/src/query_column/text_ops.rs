//! `LIKE` on the column types that hold text.

use crate::column::{Date, Text, Time, Uuid, Varchar};
use crate::expr::{Expr, Scalar};
use crate::value::Value;

use super::column::Column;

pub trait TextOps {
  /// `"<table>"."<column>" LIKE ?`, where `%` and `_` in the pattern are
  /// wildcards.
  fn like<V: Into<Value>>(&self, val: V) -> Expr;

  /// `"<table>"."<column>" LIKE ? ESCAPE ?`, so a `%` or `_` the pattern
  /// prefixes with `escape` matches literally.
  ///
  /// Build the pattern with
  /// [`like_pattern_literal`](crate::expr::like_pattern_literal); the escape
  /// character is bound, never interpolated.
  fn like_escape<V: Into<Value>>(&self, val: V, escape: char) -> Expr;
}

macro_rules! impl_text_ops {
  ($($ty:ty),+) => {
    $(
      impl TextOps for Column<$ty> {
        fn like<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "LIKE", val.into())
        }

        fn like_escape<V: Into<Value>>(&self, val: V, escape: char) -> Expr {
          Scalar::col(self).like_escape(Scalar::bind(val.into()), escape)
        }
      }
    )+
  };
}

impl_text_ops!(Text, Uuid, Date, Time);

impl<const N: u32> TextOps for Column<Varchar<N>> {
  fn like<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "LIKE", val.into())
  }

  fn like_escape<V: Into<Value>>(&self, val: V, escape: char) -> Expr {
    Scalar::col(self).like_escape(Scalar::bind(val.into()), escape)
  }
}
