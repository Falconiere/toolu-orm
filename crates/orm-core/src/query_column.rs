//! Column<T> and typed column operations (CommonOps, TextOps, NumericOps).

use std::marker::PhantomData;

use crate::column::{BigInt, Date, Integer, Real, SmallInt, Text, Time, Timestamp, Uuid, Varchar};
use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::{Expr, JoinCondition, OrderBy};
use crate::fts5::literal::require_sqlite;
use crate::value::Value;

// ── ColumnRef trait ───────────────────────────────────────────────────────────

pub trait ColumnRef {
  fn name(&self) -> &'static str;
  fn table(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct Column<T> {
  pub table: &'static str,
  pub name: &'static str,
  _marker: PhantomData<T>,
}

impl<T> ColumnRef for Column<T> {
  fn name(&self) -> &'static str {
    self.name
  }

  fn table(&self) -> &'static str {
    self.table
  }
}

impl<T> Column<T> {
  pub const fn new(table: &'static str, name: &'static str) -> Self {
    Self {
      table,
      name,
      _marker: PhantomData,
    }
  }

  pub fn qualified(&self) -> String {
    format!(r#""{}"."{}""#, self.table, self.name)
  }

  pub fn asc(&self) -> OrderBy {
    OrderBy {
      column: self.qualified(),
      direction: "ASC",
    }
  }

  pub fn desc(&self) -> OrderBy {
    OrderBy {
      column: self.qualified(),
      direction: "DESC",
    }
  }

  pub fn equals<U>(&self, other: &Column<U>) -> JoinCondition {
    JoinCondition {
      left: self.qualified(),
      right: other.qualified(),
    }
  }
}

// ── Traits ───────────────────────────────────────────────────────────────────

pub trait CommonOps {
  fn eq<V: Into<Value>>(&self, val: V) -> Expr;
  fn ne<V: Into<Value>>(&self, val: V) -> Expr;
  fn in_list(&self, values: &[Value]) -> Expr;
  fn not_in(&self, values: &[Value]) -> Expr;
  fn is_null(&self) -> Expr;
  fn is_not_null(&self) -> Expr;
}

pub trait TextOps {
  fn like<V: Into<Value>>(&self, val: V) -> Expr;
}

/// Full-text `MATCH` restricted to one column of an FTS5 table.
///
/// The whole-table form — [`Expr::table_match`], which searches every indexed
/// column — is the common case; this narrows the search to this column.
pub trait Fts5Ops {
  /// `"<table>"."<column>" MATCH ?` for an explicit dialect.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::Fts5UnsupportedDialect`] for [`Dialect::Postgres`]; see
  /// [`Expr::table_match_for`].
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, pattern: V) -> Result<Expr, DbCoreError>;

  /// [`Fts5Ops::matches_for`] against [`Dialect::CURRENT`].
  ///
  /// # Errors
  ///
  /// See [`Fts5Ops::matches_for`].
  fn matches<V: Into<Value>>(&self, pattern: V) -> Result<Expr, DbCoreError> {
    self.matches_for(Dialect::CURRENT, pattern)
  }
}

/// FTS5 stores every column as text and `#[fts5_table]` declares them `Text`,
/// so this is the only column type a `MATCH` can name.
impl Fts5Ops for Column<Text> {
  fn matches_for<V: Into<Value>>(&self, dialect: Dialect, pattern: V) -> Result<Expr, DbCoreError> {
    require_sqlite("MATCH", dialect)?;
    Ok(Expr::match_target(self.qualified(), pattern.into()))
  }
}

pub trait NumericOps {
  fn gt<V: Into<Value>>(&self, val: V) -> Expr;
  fn lt<V: Into<Value>>(&self, val: V) -> Expr;
  fn gte<V: Into<Value>>(&self, val: V) -> Expr;
  fn lte<V: Into<Value>>(&self, val: V) -> Expr;
  fn between<L: Into<Value>, H: Into<Value>>(&self, low: L, high: H) -> Expr;
}

// ── Blanket CommonOps impl for all Column<T> ─────────────────────────────────

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

// ── TextOps impls ─────────────────────────────────────────────────────────────

macro_rules! impl_text_ops {
  ($($ty:ty),+) => {
    $(
      impl TextOps for Column<$ty> {
        fn like<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "LIKE", val.into())
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
}

// ── NumericOps impls ──────────────────────────────────────────────────────────

macro_rules! impl_numeric_ops {
  ($($ty:ty),+) => {
    $(
      impl NumericOps for Column<$ty> {
        fn gt<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), ">", val.into())
        }

        fn lt<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "<", val.into())
        }

        fn gte<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), ">=", val.into())
        }

        fn lte<V: Into<Value>>(&self, val: V) -> Expr {
          Expr::comparison(self.qualified(), "<=", val.into())
        }

        fn between<L: Into<Value>, H: Into<Value>>(&self, low: L, high: H) -> Expr {
          Expr::between(self.qualified(), low.into(), high.into())
        }
      }
    )+
  };
}

impl_numeric_ops!(Integer, Real, BigInt, SmallInt, Timestamp, Date, Time);
