//! The `#[column(...)]` helper attribute: reading its keys, and removing it.
//!
//! Both halves belong to the same attribute's lifetime. `#[column(...)]` is a
//! helper attribute of `#[table]`, so it has no definition of its own; once its
//! keys have been read, it must be stripped from the struct before the struct
//! is re-emitted, or the generated code fails with "cannot find attribute
//! `column`". `parse_column_attrs` does the reading and `strip_column_attrs`
//! the removal.
//!
//! Sits beside [`super::table_attrs`], which does the same for `#[table(...)]`.

use syn::{Attribute, Fields, ItemStruct, Lit, Result};

use super::column_flags::ColumnFlags;
use super::vec0_column::{parse_vec0_meta, Vec0ColumnInput};

/// One field's `#[column(...)]` keys, before they are folded into a
/// [`super::ColumnInput`] alongside the field's name and type.
pub(super) struct ColumnAttrs {
  pub(super) column_type: Option<String>,
  pub(super) flags: ColumnFlags,
  pub(super) default: Option<String>,
  pub(super) references: Option<String>,
  pub(super) on_delete: Option<String>,
  pub(super) on_update: Option<String>,
  pub(super) check: Option<String>,
  pub(super) vec0: Vec0ColumnInput,
}

/// Reads every `#[column(...)]` attribute on one field.
pub(super) fn parse_column_attrs(attrs: &[Attribute]) -> Result<ColumnAttrs> {
  let mut result = ColumnAttrs {
    column_type: None,
    flags: ColumnFlags::default(),
    default: None,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    vec0: Vec0ColumnInput::default(),
  };

  for attr in attrs {
    if !attr.path().is_ident("column") {
      continue;
    }
    attr.parse_nested_meta(|meta| apply_column_meta(&meta, &mut result))?;
  }

  Ok(result)
}

fn apply_column_meta(
  meta: &syn::meta::ParseNestedMeta<'_>,
  result: &mut ColumnAttrs,
) -> Result<()> {
  if meta.path.is_ident("as_text") {
    if result.check.is_some() {
      return Err(meta.error(
        "cannot combine check = \"...\" with as_text; pick one source of truth for the column CHECK",
      ));
    }
    result.flags.set_as_text();
    return Ok(());
  }
  if meta.path.is_ident("column_type") {
    parse_column_type(meta, result)?;
    return Ok(());
  }
  if meta.path.is_ident("primary_key") {
    result.flags.set_primary_key();
    return Ok(());
  }
  if meta.path.is_ident("autoincrement") {
    result.flags.set_autoincrement();
    return Ok(());
  }
  if meta.path.is_ident("not_null") {
    result.flags.set_not_null();
    return Ok(());
  }
  if meta.path.is_ident("unique") {
    result.flags.set_unique();
    return Ok(());
  }
  if meta.path.is_ident("unindexed") {
    result.flags.set_unindexed();
    return Ok(());
  }
  if meta.path.is_ident("default") {
    result.default = Some(parse_str_lit(meta)?);
    return Ok(());
  }
  if meta.path.is_ident("references") {
    result.references = Some(parse_str_lit(meta)?);
    return Ok(());
  }
  if meta.path.is_ident("on_delete") {
    result.on_delete = Some(parse_str_lit(meta)?);
    return Ok(());
  }
  if meta.path.is_ident("on_update") {
    result.on_update = Some(parse_str_lit(meta)?);
    return Ok(());
  }
  if meta.path.is_ident("check") {
    if result.flags.as_text() {
      return Err(meta.error(
        "cannot combine check = \"...\" with as_text; pick one source of truth for the column CHECK",
      ));
    }
    result.check = Some(parse_str_lit(meta)?);
    return Ok(());
  }
  parse_vec0_meta(meta, &mut result.vec0, &mut result.flags)
}

fn parse_str_lit(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<String> {
  let value = meta.value()?;
  let lit: Lit = value.parse()?;
  if let Lit::Str(s) = lit {
    return Ok(s.value());
  }
  Err(meta.error("expected a string literal"))
}

fn parse_column_type(
  meta: &syn::meta::ParseNestedMeta<'_>,
  result: &mut ColumnAttrs,
) -> Result<()> {
  let value = meta.value()?;
  let expr: syn::Expr = value.parse()?;
  if let syn::Expr::Lit(expr_lit) = expr {
    if let Lit::Str(s) = expr_lit.lit {
      result.column_type = Some(s.value());
    }
  } else if let syn::Expr::Path(expr_path) = expr {
    if let Some(segment) = expr_path.path.segments.last() {
      result.column_type = Some(segment.ident.to_string());
    }
  }
  Ok(())
}

/// Removes `#[column(...)]` helper attributes from struct fields before re-emission.
/// This prevents "cannot find attribute `column`" errors in the generated code.
pub fn strip_column_attrs(mut item: ItemStruct) -> ItemStruct {
  if let Fields::Named(ref mut fields) = item.fields {
    for field in &mut fields.named {
      field.attrs.retain(|attr| !attr.path().is_ident("column"));
    }
  }
  item
}
