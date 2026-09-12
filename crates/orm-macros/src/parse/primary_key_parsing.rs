//! Parsing of table-level `#[primary_key(col, …)]`.

use syn::{Attribute, ItemStruct, Result};

/// Extracts and strips `#[primary_key(a, b)]` from the struct.
///
/// Returns the column names in declaration order. Absent attribute → empty vec.
pub fn parse_primary_key_attr(item: &mut ItemStruct) -> Result<Vec<String>> {
  let mut primary_key: Option<Vec<String>> = None;
  let mut remaining_attrs = Vec::new();

  for attr in &item.attrs {
    if !attr.path().is_ident("primary_key") {
      remaining_attrs.push(attr.clone());
      continue;
    }
    if primary_key.is_some() {
      return Err(syn::Error::new_spanned(
        attr,
        "duplicate #[primary_key(...)] on the same table",
      ));
    }
    primary_key = Some(parse_columns(attr)?);
  }
  item.attrs = remaining_attrs;
  Ok(primary_key.unwrap_or_default())
}

fn parse_columns(attr: &Attribute) -> Result<Vec<String>> {
  let args = attr
    .parse_args_with(syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)?;
  if args.is_empty() {
    return Err(syn::Error::new_spanned(
      attr,
      "#[primary_key(...)] needs at least one column",
    ));
  }
  args
    .iter()
    .map(|expr| {
      if let syn::Expr::Path(p) = expr {
        p.path
          .get_ident()
          .map(|i| i.to_string())
          .ok_or_else(|| syn::Error::new_spanned(expr, "expected column identifier"))
      } else {
        Err(syn::Error::new_spanned(expr, "expected column identifier"))
      }
    })
    .collect()
}
