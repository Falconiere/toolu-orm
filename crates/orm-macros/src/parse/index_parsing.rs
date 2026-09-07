//! Parsing of `#[index]` and `#[unique_index]` attributes and the [`TableInput`] type.
//!
//! # Public API
//!
//! - [`IndexInput`] — parsed index definition
//! - [`TableInput`] — aggregated table macro input
//! - [`parse_index_attrs`] — extract index attrs from a struct
//!
//! # Usage
//!
//! ```ignore
//! let indexes = parse_index_attrs(&mut item_struct)?;
//! ```

use syn::{Attribute, Ident, ItemStruct, Result};

use super::column_parsing::ColumnInput;

pub struct IndexInput {
  pub name: String,
  pub columns: Vec<String>,
  pub unique: bool,
}

pub struct TableInput {
  pub table_name: String,
  pub strict: bool,
  pub struct_name: Ident,
  pub columns: Vec<ColumnInput>,
  pub indexes: Vec<IndexInput>,
}

pub fn parse_index_attrs(item: &mut ItemStruct) -> Result<Vec<IndexInput>> {
  let mut indexes = Vec::new();
  let mut remaining_attrs = Vec::new();

  for attr in &item.attrs {
    if attr.path().is_ident("index") {
      indexes.push(parse_index_attr(attr, false)?);
    } else if attr.path().is_ident("unique_index") {
      indexes.push(parse_index_attr(attr, true)?);
    } else {
      remaining_attrs.push(attr.clone());
    }
  }
  item.attrs = remaining_attrs;
  Ok(indexes)
}

fn parse_index_attr(attr: &Attribute, unique: bool) -> Result<IndexInput> {
  let args = attr
    .parse_args_with(syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)?;
  let mut iter = args.iter();
  let name = match iter.next() {
    Some(syn::Expr::Lit(syn::ExprLit {
      lit: syn::Lit::Str(s),
      ..
    })) => s.value(),
    _ => {
      return Err(syn::Error::new_spanned(
        attr,
        "first arg must be index name string",
      ))
    },
  };
  let columns: Vec<String> = iter
    .map(|expr| {
      if let syn::Expr::Path(p) = expr {
        Ok(
          p.path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default(),
        )
      } else {
        Err(syn::Error::new_spanned(expr, "expected column identifier"))
      }
    })
    .collect::<Result<_>>()?;
  Ok(IndexInput {
    name,
    columns,
    unique,
  })
}
