//! The `#[table(...)]` arguments, parsed where every other parser lives.

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{Expr, Lit, Meta};

/// Parses `#[table(name = "table_name")]` or `#[table(name = "table_name", strict = true)]`.
/// Returns (table_name, strict). Strict defaults to false (standard SQLite compatibility).
pub fn parse_table_attrs(attr: TokenStream) -> syn::Result<(String, bool)> {
  let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
  let metas = parser.parse(attr)?;

  let mut table_name = None;
  let mut strict = false; // default: non-strict for SQLite/libsql compatibility

  for meta in &metas {
    let Meta::NameValue(nv) = meta else {
      return Err(syn::Error::new_spanned(
        meta,
        "expected name = value pairs in #[table(...)]",
      ));
    };
    if nv.path.is_ident("name") {
      let Expr::Lit(expr_lit) = &nv.value else {
        return Err(syn::Error::new_spanned(
          &nv.value,
          "expected a string literal",
        ));
      };
      let Lit::Str(s) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(
          &expr_lit.lit,
          "expected a string literal",
        ));
      };
      table_name = Some(s.value());
    } else if nv.path.is_ident("strict") {
      let Expr::Lit(expr_lit) = &nv.value else {
        return Err(syn::Error::new_spanned(
          &nv.value,
          "expected a bool literal",
        ));
      };
      let Lit::Bool(b) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(
          &expr_lit.lit,
          "expected true or false",
        ));
      };
      strict = b.value();
    } else {
      return Err(syn::Error::new_spanned(
        &nv.path,
        "unknown attribute, expected `name` or `strict`",
      ));
    }
  }

  let Some(name) = table_name else {
    return Err(syn::Error::new(
      proc_macro2::Span::call_site(),
      "missing `name` in #[table(name = \"...\")]",
    ));
  };

  Ok((name, strict))
}
