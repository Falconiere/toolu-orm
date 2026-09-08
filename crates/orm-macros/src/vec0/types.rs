//! The marker type a field declares, resolved to the enum variant `vec0`
//! accepts in that column position.
//!
//! Each position has its own list because `vec0` does: no `boolean` key, no
//! `blob` metadata column, no `boolean` auxiliary column.

use proc_macro2::{Ident, Span};

use crate::parse::{ColumnInput, TypeSpec};

use super::columns::err;

pub fn key_type(column: &ColumnInput) -> syn::Result<Ident> {
  resolve(
    column,
    "a vec0 key",
    &[("Text", "Text"), ("Integer", "Integer")],
  )
}

pub fn metadata_type(column: &ColumnInput) -> syn::Result<Ident> {
  resolve(
    column,
    "a vec0 metadata column",
    &[
      ("Text", "Text"),
      ("Integer", "Integer"),
      ("Real", "Float"),
      ("Boolean", "Boolean"),
    ],
  )
}

pub fn auxiliary_type(column: &ColumnInput) -> syn::Result<Ident> {
  resolve(
    column,
    "a vec0 auxiliary column",
    &[
      ("Text", "Text"),
      ("Integer", "Integer"),
      ("Real", "Float"),
      ("Blob", "Blob"),
    ],
  )
}

/// `allowed` maps the declared marker type to the variant it becomes.
fn resolve(column: &ColumnInput, position: &str, allowed: &[(&str, &str)]) -> syn::Result<Ident> {
  let declared = match &column.type_spec {
    TypeSpec::Simple(name) => name.as_str(),
    TypeSpec::Varchar(_) => "Varchar",
    TypeSpec::Char(_) => "Char",
  };
  let variant = allowed
    .iter()
    .find(|(marker, _)| *marker == declared)
    .map(|(_, variant)| *variant)
    .ok_or_else(|| {
      let markers: Vec<&str> = allowed.iter().map(|(marker, _)| *marker).collect();
      err(
        column,
        format!(
          "`{}` is {position}, so its type must be one of {}, not `{declared}`",
          column.field_name,
          markers.join(", ")
        ),
      )
    })?;
  Ok(Ident::new(variant, Span::call_site()))
}
