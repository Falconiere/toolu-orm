mod column_enum;
mod expand;
mod from_row;
mod from_row_expand;
mod parse;
mod paths;
mod relational;
mod view;

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{parse_macro_input, Expr, ItemStruct, Lit, Meta};

#[proc_macro_attribute]
pub fn table(attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut item_struct = parse_macro_input!(item as ItemStruct);

  let (table_name, strict) = match parse_table_attrs(attr) {
    Ok(attrs) => attrs,
    Err(e) => return e.to_compile_error().into(),
  };

  // Parse and strip #[index] / #[unique_index] attrs from the struct
  let indexes = match parse::parse_index_attrs(&mut item_struct) {
    Ok(idxs) => idxs,
    Err(e) => return e.to_compile_error().into(),
  };

  // Parse and strip #[view] attrs from the struct
  let views = match parse_view_attrs(&mut item_struct) {
    Ok(v) => v,
    Err(e) => return e.to_compile_error().into(),
  };

  let columns = match parse::parse_struct(&item_struct) {
    Ok(cols) => cols,
    Err(e) => return e.to_compile_error().into(),
  };

  let input = parse::TableInput {
    table_name,
    strict,
    struct_name: item_struct.ident.clone(),
    columns,
    indexes,
  };

  let expanded = expand::expand(&input);
  let columns_mod = expand::expand_columns_module(&input);
  let builder_methods = expand::expand_builder_methods(&input);
  let vis = &item_struct.vis;
  let view_structs: Vec<proc_macro2::TokenStream> = views
    .iter()
    .map(|v| view::generate_view_struct(v, &input.columns, vis))
    .collect();
  let clean_struct = parse::strip_column_attrs(item_struct);

  let output = quote::quote! {
      #clean_struct
      #expanded
      #builder_methods
      #columns_mod
      #(#view_structs)*
  };

  output.into()
}

fn parse_view_attrs(item: &mut ItemStruct) -> syn::Result<Vec<view::ViewInput>> {
  let mut views = Vec::new();
  let mut remaining_attrs = Vec::new();
  for attr in &item.attrs {
    if attr.path().is_ident("view") {
      views.push(view::parse_view_attr(attr)?);
    } else {
      remaining_attrs.push(attr.clone());
    }
  }
  item.attrs = remaining_attrs;
  Ok(views)
}

/// Parses `#[table(name = "table_name")]` or `#[table(name = "table_name", strict = true)]`.
/// Returns (table_name, strict). Strict defaults to false (standard SQLite compatibility).
fn parse_table_attrs(attr: TokenStream) -> syn::Result<(String, bool)> {
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

#[proc_macro_derive(ColumnEnum)]
pub fn derive_column_enum(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  match column_enum::expand_column_enum(&input) {
    Ok(tokens) => tokens.into(),
    Err(e) => e.to_compile_error().into(),
  }
}

#[proc_macro_derive(FromRow, attributes(from_row))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  match from_row::expand_from_row(&input) {
    Ok(tokens) => tokens.into(),
    Err(e) => e.to_compile_error().into(),
  }
}

#[proc_macro_derive(Relational, attributes(relational, has_many, belongs_to, many_to_many))]
pub fn derive_relational(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  match relational::parse_relational(&input) {
    Ok(parsed) => expand::expand_relational(&parsed).into(),
    Err(e) => e.to_compile_error().into(),
  }
}
