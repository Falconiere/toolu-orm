//! Parsing of struct fields into column definitions.
//!
//! One field becomes one [`ColumnInput`]: its name, its type read by
//! [`super::column_type_spec`], and its `#[column(...)]` keys read by
//! [`super::column_attrs`].
//!
//! ```ignore
//! let columns = parse_struct(&item_struct)?;
//! ```

use syn::{Error, Fields, ItemStruct, Result};

use super::column_attrs::parse_column_attrs;
use super::column_flags::ColumnFlags;
use super::column_type_spec::{extract_type_spec, TypeSpec};
use super::vec0_column::Vec0ColumnInput;

/// One column of a `#[table]` struct, as written on the field.
pub struct ColumnInput {
  pub field_name: String,
  pub type_spec: TypeSpec,
  pub original_type: syn::Type,
  pub explicit_column_type: Option<String>,
  pub flags: ColumnFlags,
  pub default: Option<String>,
  pub references: Option<String>,
  pub on_delete: Option<String>,
  pub on_update: Option<String>,
  /// Raw CHECK body from `#[column(check = "...")]`, without the `CHECK (...)` wrapper.
  pub check: Option<String>,
  /// Read only by `#[vec0_table]`; empty for every other column.
  pub vec0: Vec0ColumnInput,
}

/// Walks a struct's named fields, in declaration order, into column inputs.
pub fn parse_struct(item: &ItemStruct) -> Result<Vec<ColumnInput>> {
  let Fields::Named(fields) = &item.fields else {
    return Err(Error::new_spanned(
      item,
      "table structs must have named fields",
    ));
  };

  let mut columns = Vec::new();
  for field in &fields.named {
    let Some(ident) = &field.ident else { continue };
    let field_name = ident.to_string();
    let type_spec = extract_type_spec(&field.ty)?;
    let col_attrs = parse_column_attrs(&field.attrs)?;
    columns.push(ColumnInput {
      field_name,
      type_spec,
      original_type: field.ty.clone(),
      explicit_column_type: col_attrs.column_type,
      flags: col_attrs.flags,
      default: col_attrs.default,
      references: col_attrs.references,
      on_delete: col_attrs.on_delete,
      on_update: col_attrs.on_update,
      check: col_attrs.check,
      vec0: col_attrs.vec0,
    });
  }
  Ok(columns)
}
