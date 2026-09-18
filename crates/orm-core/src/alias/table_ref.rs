//! [`TableRef`] — a table in a `FROM` / `JOIN` slot, optionally under an alias.

use crate::query_column::Column;

use super::aliased_column::AliasedColumn;
use super::quoting::quote_ident;

/// A table named in a `FROM` or `JOIN` slot, with an optional alias.
///
/// Keeping the alias out of the name is what makes a self-join expressible:
/// `TableRef::aliased("memories", "old")` renders `"memories" AS "old"`, where
/// the whole string in one identifier — `"memories old"` — names nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRef {
  table: String,
  alias: Option<String>,
}

impl TableRef {
  /// The table under its own name.
  pub fn new(table: impl Into<String>) -> Self {
    Self {
      table: table.into(),
      alias: None,
    }
  }

  /// The table under `alias`, which is what its columns are qualified by.
  pub fn aliased(table: impl Into<String>, alias: impl Into<String>) -> Self {
    Self {
      table: table.into(),
      alias: Some(alias.into()),
    }
  }

  /// The base table name, never the alias.
  ///
  /// This is the name an error message should carry, because it is the relation
  /// that exists in the schema.
  pub fn table(&self) -> &str {
    &self.table
  }

  /// The alias, when there is one.
  pub fn alias(&self) -> Option<&str> {
    self.alias.as_deref()
  }

  /// What columns of this table are qualified by: the alias, else the table.
  ///
  /// SQL hides the original name once a table is aliased, so this is the only
  /// qualifier the rest of the statement may use.
  pub fn qualifier(&self) -> &str {
    self.alias.as_deref().unwrap_or(&self.table)
  }

  /// `"table"` or `"table" AS "alias"`, both parts quoted.
  pub fn to_sql(&self) -> String {
    let table = quote_ident(&self.table);
    match &self.alias {
      Some(alias) => format!("{table} AS {}", quote_ident(alias)),
      None => table,
    }
  }

  /// `column` of this table, qualified by [`TableRef::qualifier`].
  ///
  /// The marker type rides along, so an [`AliasedColumn`] keeps the same typed
  /// predicates the original [`Column`] had.
  pub fn column<T>(&self, column: &Column<T>) -> AliasedColumn<T> {
    AliasedColumn::new(self.qualifier(), column.name)
  }
}

impl From<&str> for TableRef {
  fn from(table: &str) -> Self {
    Self::new(table)
  }
}

impl From<String> for TableRef {
  fn from(table: String) -> Self {
    Self::new(table)
  }
}

/// So a builder can take `&table_ref` without the caller spelling out a clone
/// at each of the several slots one alias is named in.
impl From<&TableRef> for TableRef {
  fn from(table: &TableRef) -> Self {
    table.clone()
  }
}
