//! [`AliasedColumn`] — a column carrying the qualifier of an aliased table.

use std::fmt;
use std::marker::PhantomData;

use crate::column::{BigInt, Date, Integer, Real, SmallInt, Text, Time, Timestamp, Uuid, Varchar};
use crate::expr::{Expr, OrderBy, Scalar};
use crate::query_column::{tag_column_bind, CommonOps, NumericOps, TextOps};
use crate::value::Value;

use super::quoting::quote_ident;

/// A column of a table named under an alias.
///
/// `Column<T>` is built by a `const fn` over `&'static str`, which an alias
/// chosen at runtime cannot be; this is its owned-qualifier twin. It carries
/// the same marker `T`, so the typed predicates come along.
///
/// Build one with [`TableRef::column`](super::TableRef::column). Neither
/// `Fts5Ops` nor `Vec0Ops` is implemented: FTS5's auxiliary functions and
/// `vec0`'s hidden `k` column are not addressable through an alias.
pub struct AliasedColumn<T> {
  qualifier: String,
  name: String,
  marker: PhantomData<T>,
}

/// Written out rather than derived: the marker types are bare unit structs, so
/// a derive's `T: Clone` bound would leave `AliasedColumn<Text>` uncloneable.
impl<T> Clone for AliasedColumn<T> {
  fn clone(&self) -> Self {
    Self {
      qualifier: self.qualifier.clone(),
      name: self.name.clone(),
      marker: PhantomData,
    }
  }
}

/// Written out for the same reason as [`Clone`].
impl<T> fmt::Debug for AliasedColumn<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("AliasedColumn")
      .field("qualifier", &self.qualifier)
      .field("name", &self.name)
      .finish()
  }
}

impl<T> AliasedColumn<T> {
  /// `qualifier` is the alias (or table) the column is addressed through.
  pub(super) fn new(qualifier: &str, name: &str) -> Self {
    Self {
      qualifier: qualifier.to_owned(),
      name: name.to_owned(),
      marker: PhantomData,
    }
  }

  /// The alias (or table) this column is addressed through.
  pub fn qualifier(&self) -> &str {
    &self.qualifier
  }

  /// The column name on the underlying table.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// `"qualifier"."name"`, both parts quoted.
  pub fn qualified(&self) -> String {
    format!(
      "{}.{}",
      quote_ident(&self.qualifier),
      quote_ident(&self.name)
    )
  }

  /// The column as a [`Scalar`], for a projection or a computed term.
  pub fn scalar(&self) -> Scalar {
    Scalar::sql(self.qualified())
  }

  /// `ORDER BY "qualifier"."name" ASC`.
  pub fn asc(&self) -> OrderBy {
    self.scalar().asc()
  }

  /// `ORDER BY "qualifier"."name" DESC`.
  pub fn desc(&self) -> OrderBy {
    self.scalar().desc()
  }
}

impl<T: 'static> CommonOps for AliasedColumn<T> {
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

macro_rules! impl_text_ops {
  ($($ty:ty),+) => {
    $(
      impl TextOps for AliasedColumn<$ty> {
        fn like<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "LIKE", val.into())
        }

        fn like_escape<V: Into<Value>>(&self, val: V, escape: char) -> Expr {
          self.scalar().like_escape(Scalar::bind(val.into()), escape)
        }
      }
    )+
  };
}

impl_text_ops!(Text, Uuid, Date, Time);

impl<const N: u32> TextOps for AliasedColumn<Varchar<N>> {
  fn like<V: Into<Value>>(&self, val: V) -> Expr {
    Expr::comparison(self.qualified(), "LIKE", val.into())
  }

  fn like_escape<V: Into<Value>>(&self, val: V, escape: char) -> Expr {
    self.scalar().like_escape(Scalar::bind(val.into()), escape)
  }
}

macro_rules! impl_numeric_ops {
  ($($ty:ty),+) => {
    $(
      impl NumericOps for AliasedColumn<$ty> {
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
