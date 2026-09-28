//! Compile-time checks of `#[foreign_key(...)]` against the struct it sits on.

use std::collections::BTreeSet;

use syn::{Error, Result};

use super::{ColumnInput, ForeignKeyInput};

/// Every member column is a field of the struct, and every foreign-key name
/// on the table is unique — including the `fk_<table>_<col>` names the
/// column-level `references` take.
pub fn validate_foreign_keys(
  table_name: &str,
  foreign_keys: &[ForeignKeyInput],
  columns: &[ColumnInput],
) -> Result<()> {
  let mut names: BTreeSet<String> = columns
    .iter()
    .filter(|c| c.references.is_some())
    .map(|c| format!("fk_{table_name}_{}", c.field_name))
    .collect();
  for fk in foreign_keys {
    for member in &fk.columns {
      if !columns.iter().any(|c| member == &c.field_name) {
        return Err(Error::new_spanned(
          member,
          format!("`{member}` is not a field of this struct"),
        ));
      }
    }
    let name = fk.name(table_name);
    if !names.insert(name.clone()) {
      let message = format!("duplicate foreign key name \"{name}\" on the same table");
      return Err(match &fk.name {
        Some(lit) => Error::new_spanned(lit, message),
        None => Error::new_spanned(&fk.attr, message),
      });
    }
  }
  Ok(())
}
