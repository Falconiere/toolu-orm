//! Parsing of struct fields and `#[column(...)]` attributes.
//!
//! # Public API
//!
//! - [`ColumnInput`] — parsed column definition
//! - [`TypeSpec`] — simple or varchar type specification
//! - [`parse_struct`] — parse named fields into column inputs
//! - [`strip_column_attrs`] — remove `#[column]` attrs before re-emission
//!
//! # Usage
//!
//! ```ignore
//! let columns = parse_struct(&item_struct)?;
//! let clean = strip_column_attrs(item_struct);
//! ```

use syn::{Attribute, Error, Fields, ItemStruct, Lit, Result};

use super::column_flags::ColumnFlags;
use super::vec0_column::{parse_vec0_meta, Vec0ColumnInput};

pub enum TypeSpec {
  Simple(String),
  Varchar(u32),
  Char(u32),
}

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

struct ColumnAttrs {
  column_type: Option<String>,
  flags: ColumnFlags,
  default: Option<String>,
  references: Option<String>,
  on_delete: Option<String>,
  on_update: Option<String>,
  check: Option<String>,
  vec0: Vec0ColumnInput,
}

fn parse_column_attrs(attrs: &[Attribute]) -> Result<ColumnAttrs> {
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

fn extract_type_spec(ty: &syn::Type) -> Result<TypeSpec> {
  if let syn::Type::Path(type_path) = ty {
    if let Some(segment) = type_path.path.segments.last() {
      let name = segment.ident.to_string();
      // Check for Varchar<N> generic syntax
      if name == "Varchar" {
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
          if let Some(syn::GenericArgument::Const(syn::Expr::Lit(lit))) = args.args.first() {
            if let syn::Lit::Int(int_lit) = &lit.lit {
              let n: u32 = int_lit
                .base10_parse()
                .map_err(|_parse_err| Error::new_spanned(int_lit, "expected u32 for Varchar<N>"))?;
              return Ok(TypeSpec::Varchar(n));
            }
          }
        }
        return Err(Error::new_spanned(
          segment,
          "Varchar requires a const parameter: Varchar<255>",
        ));
      }
      if name == "Char" {
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
          if let Some(syn::GenericArgument::Const(syn::Expr::Lit(lit))) = args.args.first() {
            if let syn::Lit::Int(int_lit) = &lit.lit {
              let n: u32 = int_lit
                .base10_parse()
                .map_err(|_parse_err| Error::new_spanned(int_lit, "expected u32 for Char<N>"))?;
              return Ok(TypeSpec::Char(n));
            }
          }
        }
        return Err(Error::new_spanned(
          segment,
          "Char requires a const parameter: Char<10>",
        ));
      }
      return Ok(TypeSpec::Simple(name));
    }
  }
  Err(Error::new_spanned(ty, "unsupported column type"))
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
