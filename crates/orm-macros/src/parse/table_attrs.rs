//! The `#[table(...)]` arguments, parsed where every other parser lives.

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{Expr, Lit, Meta};

/// What `#[table(...)]` declared.
pub struct TableAttrs {
  pub name: String,
  /// `strict = true`; defaults to false for SQLite/libsql compatibility.
  pub strict: bool,
  /// `rls = "enable"` → `Some(false)`, `rls = "force"` → `Some(true)`,
  /// absent → `None`.
  pub rls: Option<bool>,
}

const KNOWN_KEYS: &str = "unknown attribute, expected `name`, `strict` or `rls`";

/// Parses `#[table(name = "table_name")]` with the optional `strict = true`
/// and `rls = "enable" | "force"` keys.
pub fn parse_table_attrs(attr: TokenStream) -> syn::Result<TableAttrs> {
  let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
  let metas = parser.parse(attr)?;

  let mut table_name = None;
  let mut strict = false;
  let mut rls = None;

  for meta in &metas {
    let Meta::NameValue(nv) = meta else {
      return Err(syn::Error::new_spanned(
        meta,
        "expected name = value pairs in #[table(...)]",
      ));
    };
    if nv.path.is_ident("name") {
      table_name = Some(string_value(&nv.value)?);
    } else if nv.path.is_ident("strict") {
      strict = bool_value(&nv.value)?;
    } else if nv.path.is_ident("rls") {
      rls = Some(rls_value(&nv.value)?);
    } else {
      return Err(syn::Error::new_spanned(&nv.path, KNOWN_KEYS));
    }
  }

  let Some(name) = table_name else {
    return Err(syn::Error::new(
      proc_macro2::Span::call_site(),
      "missing `name` in #[table(name = \"...\")]",
    ));
  };

  Ok(TableAttrs { name, strict, rls })
}

fn string_value(expr: &Expr) -> syn::Result<String> {
  let Expr::Lit(expr_lit) = expr else {
    return Err(syn::Error::new_spanned(expr, "expected a string literal"));
  };
  let Lit::Str(s) = &expr_lit.lit else {
    return Err(syn::Error::new_spanned(
      &expr_lit.lit,
      "expected a string literal",
    ));
  };
  Ok(s.value())
}

fn bool_value(expr: &Expr) -> syn::Result<bool> {
  let Expr::Lit(expr_lit) = expr else {
    return Err(syn::Error::new_spanned(expr, "expected a bool literal"));
  };
  let Lit::Bool(b) = &expr_lit.lit else {
    return Err(syn::Error::new_spanned(
      &expr_lit.lit,
      "expected true or false",
    ));
  };
  Ok(b.value())
}

/// `"enable"` keeps the owner exempt; `"force"` binds the owner too.
fn rls_value(expr: &Expr) -> syn::Result<bool> {
  match string_value(expr)?.as_str() {
    "enable" => Ok(false),
    "force" => Ok(true),
    _ => Err(syn::Error::new_spanned(
      expr,
      "expected `rls = \"enable\"` or `rls = \"force\"`",
    )),
  }
}
