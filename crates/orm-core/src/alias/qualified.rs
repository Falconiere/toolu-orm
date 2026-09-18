//! [`QualifiedColumn`] — what a `Column<T>` and an `AliasedColumn<T>` share.

use crate::query_column::Column;

use super::aliased_column::AliasedColumn;

/// A column reference that renders qualified by its table or alias.
///
/// Object-safe on purpose: `&dyn QualifiedColumn` is what a projection list
/// takes, so one slice can mix plain and aliased columns.
pub trait QualifiedColumn {
  /// `"qualifier"."name"`, both parts quoted.
  fn qualified(&self) -> String;
}

impl<T> QualifiedColumn for Column<T> {
  fn qualified(&self) -> String {
    Column::qualified(self)
  }
}

impl<T> QualifiedColumn for AliasedColumn<T> {
  fn qualified(&self) -> String {
    AliasedColumn::qualified(self)
  }
}
