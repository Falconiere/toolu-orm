//! How a field's Rust type is written down for the schema.
//!
//! `Varchar<N>` and `Char<N>` carry a length the schema needs, so they are read
//! off the generic argument rather than kept as a bare name; everything else is
//! resolved later, against the target dialect.

use syn::{Error, Result};

/// A column's declared type, with the length `Varchar`/`Char` carry.
pub enum TypeSpec {
  Simple(String),
  Varchar(u32),
  Char(u32),
}

/// Reads one struct field's Rust type into a [`TypeSpec`].
pub(super) fn extract_type_spec(ty: &syn::Type) -> Result<TypeSpec> {
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
