//! Compile-time checks for primary-key / autoincrement declarations.

use syn::Result;

use super::{ColumnInput, TypeSpec};

/// Refuses `#[primary_key(...)]` combined with any `#[column(primary_key)]`.
pub fn reject_mixed_primary_keys(primary_key: &[String], columns: &[ColumnInput]) -> Result<()> {
  if primary_key.is_empty() {
    return Ok(());
  }
  for column in columns {
    if column.flags.primary_key() {
      return Err(syn::Error::new_spanned(
        &column.original_type,
        format!(
          "`{}`: cannot combine #[primary_key(...)] with #[column(primary_key)] on the same table",
          column.field_name
        ),
      ));
    }
  }
  Ok(())
}

/// `autoincrement` requires `primary_key` on an `Integer` column.
pub fn validate_autoincrement(columns: &[ColumnInput]) -> Result<()> {
  for column in columns {
    if !column.flags.autoincrement() {
      continue;
    }
    if !column.flags.primary_key() {
      return Err(syn::Error::new_spanned(
        &column.original_type,
        format!(
          "`{}`: autoincrement requires #[column(primary_key)]",
          column.field_name
        ),
      ));
    }
    if !is_integer_column(column) {
      return Err(syn::Error::new_spanned(
        &column.original_type,
        format!(
          "`{}`: autoincrement is only valid on Integer columns",
          column.field_name
        ),
      ));
    }
  }
  Ok(())
}

fn is_integer_column(column: &ColumnInput) -> bool {
  if column.explicit_column_type.as_deref() == Some("Integer") {
    return true;
  }
  matches!(&column.type_spec, TypeSpec::Simple(name) if name == "Integer")
}
