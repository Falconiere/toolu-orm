//! Everything `#[vec0_table]` does between the raw token streams.

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{ItemStruct, Meta};

use crate::expand::{expand_builder_methods, expand_columns_module};
use crate::parse::{parse_struct, strip_column_attrs, TableInput};

use super::attrs::parse_attrs;
use super::expand::expand;

pub fn expand_vec0_table(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
  let item_struct = syn::parse::<ItemStruct>(item)?;
  let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
  let metas: Vec<Meta> = parser.parse(attr)?.into_iter().collect();
  let attrs = parse_attrs(&metas)?;

  for struct_attr in &item_struct.attrs {
    if struct_attr.path().is_ident("index") || struct_attr.path().is_ident("unique_index") {
      return Err(syn::Error::new_spanned(
        struct_attr,
        "virtual tables cannot declare indexes; remove it from #[vec0_table]",
      ));
    }
  }

  let columns = parse_struct(&item_struct)?;
  if columns.is_empty() {
    return Err(syn::Error::new_spanned(
      &item_struct,
      "a vec0 table needs at least one column",
    ));
  }

  let input = TableInput {
    table_name: attrs.table_name()?,
    strict: false,
    struct_name: item_struct.ident.clone(),
    columns,
    indexes: Vec::new(),
    primary_key: Vec::new(),
  };

  let schema_impl = expand(&input)?;
  let columns_mod = expand_columns_module(&input);
  let builder_methods = expand_builder_methods(&input);
  let clean_struct = strip_column_attrs(item_struct);

  Ok(
    quote::quote! {
      #clean_struct
      #schema_impl
      #builder_methods
      #columns_mod
    }
    .into(),
  )
}
