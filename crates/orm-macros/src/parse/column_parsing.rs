//! Parsing of struct fields and `#[column(...)]` attributes.
//!
//! # Public API
//!
//! - [`ColumnInput`] — parsed column definition
//! - [`ColumnFlags`] — bitflag constraints (primary_key, not_null, etc.)
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

/// Column constraint flags (avoids excessive bools in structs for Clippy).
#[derive(Clone, Copy, Default)]
pub struct ColumnFlags(u8);

const FLAG_PRIMARY_KEY: u8 = 1 << 0;
const FLAG_NOT_NULL: u8 = 1 << 1;
const FLAG_UNIQUE: u8 = 1 << 2;
const FLAG_AS_TEXT: u8 = 1 << 3;

impl ColumnFlags {
  pub const fn primary_key(self) -> bool {
    (self.0 & FLAG_PRIMARY_KEY) != 0
  }
  pub const fn not_null(self) -> bool {
    (self.0 & FLAG_NOT_NULL) != 0
  }
  pub const fn unique(self) -> bool {
    (self.0 & FLAG_UNIQUE) != 0
  }
  pub const fn as_text(self) -> bool {
    (self.0 & FLAG_AS_TEXT) != 0
  }
  fn set_primary_key(&mut self, v: bool) {
    if v {
      self.0 |= FLAG_PRIMARY_KEY;
    } else {
      self.0 &= !FLAG_PRIMARY_KEY;
    }
  }
  fn set_not_null(&mut self, v: bool) {
    if v {
      self.0 |= FLAG_NOT_NULL;
    } else {
      self.0 &= !FLAG_NOT_NULL;
    }
  }
  fn set_unique(&mut self, v: bool) {
    if v {
      self.0 |= FLAG_UNIQUE;
    } else {
      self.0 &= !FLAG_UNIQUE;
    }
  }
  fn set_as_text(&mut self, v: bool) {
    if v {
      self.0 |= FLAG_AS_TEXT;
    } else {
      self.0 &= !FLAG_AS_TEXT;
    }
  }
}

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
}

fn parse_column_attrs(attrs: &[Attribute]) -> Result<ColumnAttrs> {
  let mut result = ColumnAttrs {
    column_type: None,
    flags: ColumnFlags::default(),
    default: None,
    references: None,
    on_delete: None,
    on_update: None,
  };

  for attr in attrs {
    if !attr.path().is_ident("column") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("as_text") {
        result.flags.set_as_text(true);
      } else if meta.path.is_ident("column_type") {
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
      } else if meta.path.is_ident("primary_key") {
        result.flags.set_primary_key(true);
      } else if meta.path.is_ident("not_null") {
        result.flags.set_not_null(true);
      } else if meta.path.is_ident("unique") {
        result.flags.set_unique(true);
      } else if meta.path.is_ident("default") {
        let value = meta.value()?;
        let lit: Lit = value.parse()?;
        if let Lit::Str(s) = lit {
          result.default = Some(s.value());
        }
      } else if meta.path.is_ident("references") {
        let value = meta.value()?;
        let lit: Lit = value.parse()?;
        if let Lit::Str(s) = lit {
          result.references = Some(s.value());
        }
      } else if meta.path.is_ident("on_delete") {
        let value = meta.value()?;
        let lit: Lit = value.parse()?;
        if let Lit::Str(s) = lit {
          result.on_delete = Some(s.value());
        }
      } else if meta.path.is_ident("on_update") {
        let value = meta.value()?;
        let lit: Lit = value.parse()?;
        if let Lit::Str(s) = lit {
          result.on_update = Some(s.value());
        }
      }
      Ok(())
    })?;
  }

  Ok(result)
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
