//! The typed column handle every operation trait is implemented for.

use std::marker::PhantomData;

use crate::expr::{OrderBy, Scalar};

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
    Scalar::col(self).asc()
  }

  pub fn desc(&self) -> OrderBy {
    Scalar::col(self).desc()
  }
}
