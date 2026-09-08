//! Parsed `#[vec0_table(...)]` arguments.

use proc_macro2::Span;
use syn::{Expr, Lit, Meta};

use super::ident::is_vec0_ident;

#[derive(Default)]
pub struct Vec0Attrs {
  name: Option<String>,
}

pub fn parse_attrs(metas: &[Meta]) -> syn::Result<Vec0Attrs> {
  let mut attrs = Vec0Attrs::default();
  for meta in metas {
    let Meta::NameValue(nv) = meta else {
      return Err(syn::Error::new_spanned(
        meta,
        "expected name = value pairs in #[vec0_table(...)]",
      ));
    };
    let key = nv
      .path
      .get_ident()
      .ok_or_else(|| syn::Error::new_spanned(&nv.path, "expected a simple identifier"))?
      .to_string();
    if key == "name" {
      attrs.name = Some(string_value(&nv.value)?);
    } else {
      return Err(syn::Error::new_spanned(
        &nv.path,
        "unknown attribute, expected `name`",
      ));
    }
  }
  Ok(attrs)
}

impl Vec0Attrs {
  /// The table name, checked here against `vec0`'s own identifier rule: the
  /// module parses its constructor itself and has no quoting, so a name it
  /// cannot read has no escaped form and must fail the build.
  pub fn table_name(&self) -> syn::Result<String> {
    let name = self.name.clone().ok_or_else(|| {
      syn::Error::new(
        Span::call_site(),
        "missing `name` in #[vec0_table(name = \"...\")]",
      )
    })?;
    if !is_vec0_ident(&name) {
      return Err(syn::Error::new(
        Span::call_site(),
        format!(
          "`{name}` cannot be a vec0 table name: vec0 parses its own arguments and has no \
           quoting, so the name must match [A-Za-z][A-Za-z0-9_]*"
        ),
      ));
    }
    Ok(name)
  }
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
