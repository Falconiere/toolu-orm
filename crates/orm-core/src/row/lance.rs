//! Owned, driver-neutral columns projected from a Lance query.

use crate::{error::DbCoreError, value::Value};

/// A selected Lance result row without DuckDB types in the public API.
#[derive(Debug, Clone, PartialEq)]
pub struct LanceRow {
  columns: Vec<(String, Value)>,
}

impl LanceRow {
  /// Build a row from named portable values, rejecting ambiguous names.
  ///
  /// # Errors
  ///
  /// Returns `RowMapping` when two projected names compare equal under
  /// DuckDB's case-insensitive identifier lookup.
  pub fn from_columns(columns: Vec<(String, Value)>) -> Result<Self, DbCoreError> {
    let mut seen: Vec<&str> = Vec::with_capacity(columns.len());
    for (name, _) in &columns {
      if seen.iter().any(|other| other.eq_ignore_ascii_case(name)) {
        return Err(DbCoreError::RowMapping(format!(
          "ambiguous Lance result column {name}"
        )));
      }
      seen.push(name);
    }
    Ok(Self { columns })
  }

  /// Read one projected value by its case-insensitive column name.
  ///
  /// # Errors
  ///
  /// Returns `RowMapping` naming a missing column.
  pub fn get(&self, name: &str) -> Result<&Value, DbCoreError> {
    self
      .columns
      .iter()
      .find(|(column, _)| column.eq_ignore_ascii_case(name))
      .map(|(_, value)| value)
      .ok_or_else(|| DbCoreError::RowMapping(format!("missing Lance result column {name}")))
  }
}
