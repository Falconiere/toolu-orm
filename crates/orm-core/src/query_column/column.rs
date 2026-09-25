//! The typed column handle every operation trait is implemented for.

use std::marker::PhantomData;

use crate::alias::quote_ident;
use crate::expr::{OrderBy, Scalar};

/// Names exposed by a typed column.
pub trait ColumnRef {
  /// Column name without a table qualifier.
  fn name(&self) -> &'static str;
  /// Defining table name.
  fn table(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy)]
/// Typed handle for a column declared by a table.
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
  /// Construct a typed column from static table and column names.
  pub const fn new(table: &'static str, name: &'static str) -> Self {
    Self {
      table,
      name,
      _marker: PhantomData,
    }
  }

  /// Render both identifier parts with escaped double quotes.
  pub fn qualified(&self) -> String {
    format!("{}.{}", quote_ident(self.table), quote_ident(self.name))
  }

  /// Order this column ascending.
  pub fn asc(&self) -> OrderBy {
    Scalar::col(self).asc()
  }

  /// Order this column descending.
  pub fn desc(&self) -> OrderBy {
    Scalar::col(self).desc()
  }
}
