//! `#[fts5_table]`: FTS5 virtual tables declared as structs.
//!
//! Expands into a `TableSchema` impl built with
//! `toolu_orm_core::fts5::Fts5Table`, so the module arguments are rendered in
//! exactly one place and a macro-declared table cannot drift from a
//! hand-built one.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Expr, Lit, Meta};

use crate::parse::{ColumnInput, TableInput};
use crate::paths;

/// Parsed `#[fts5_table(...)]` arguments.
#[derive(Default)]
pub struct Fts5Attrs {
  name: Option<String>,
  tokenize: Option<String>,
  prefix: Option<String>,
  content: Option<String>,
  content_rowid: Option<String>,
  columnsize: Option<u8>,
  detail: Option<String>,
}

const KNOWN_KEYS: &str =
  "unknown attribute, expected `name`, `tokenize`, `prefix`, `content`, `content_rowid`, \
   `columnsize` or `detail`";

pub fn parse_attrs(metas: &[Meta]) -> syn::Result<Fts5Attrs> {
  let mut attrs = Fts5Attrs::default();
  for meta in metas {
    let Meta::NameValue(nv) = meta else {
      return Err(syn::Error::new_spanned(
        meta,
        "expected name = value pairs in #[fts5_table(...)]",
      ));
    };
    let key = nv
      .path
      .get_ident()
      .ok_or_else(|| syn::Error::new_spanned(&nv.path, "expected a simple identifier"))?
      .to_string();
    match key.as_str() {
      "name" => attrs.name = Some(string_value(&nv.value)?),
      "tokenize" => attrs.tokenize = Some(string_value(&nv.value)?),
      "prefix" => attrs.prefix = Some(string_value(&nv.value)?),
      "content" => attrs.content = Some(string_value(&nv.value)?),
      "content_rowid" => attrs.content_rowid = Some(string_value(&nv.value)?),
      "detail" => attrs.detail = Some(string_value(&nv.value)?),
      "columnsize" => attrs.columnsize = Some(columnsize_value(&nv.value)?),
      _ => return Err(syn::Error::new_spanned(&nv.path, KNOWN_KEYS)),
    }
  }
  Ok(attrs)
}

impl Fts5Attrs {
  /// The table name, the one attribute an FTS5 table cannot do without.
  pub fn table_name(&self) -> syn::Result<String> {
    self.name.clone().ok_or_else(|| {
      syn::Error::new(
        Span::call_site(),
        "missing `name` in #[fts5_table(name = \"...\")]",
      )
    })
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

fn columnsize_value(expr: &Expr) -> syn::Result<u8> {
  let Expr::Lit(expr_lit) = expr else {
    return Err(syn::Error::new_spanned(expr, "expected 0 or 1"));
  };
  let Lit::Int(int_lit) = &expr_lit.lit else {
    return Err(syn::Error::new_spanned(&expr_lit.lit, "expected 0 or 1"));
  };
  let value: u8 = int_lit.base10_parse()?;
  if value > 1 {
    return Err(syn::Error::new_spanned(int_lit, "expected 0 or 1"));
  }
  Ok(value)
}

/// The `TableSchema` impl, built through the core FTS5 builder.
pub fn expand(attrs: &Fts5Attrs, input: &TableInput) -> TokenStream {
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;
  let core = paths::core();
  let column_calls = input.columns.iter().map(|c| column_call(&core, c));
  let option_calls = option_calls(attrs);

  quote! {
    impl #core::table::TableSchema for #struct_name {
      fn table_def() -> #core::table::TableDef {
        #core::fts5::Fts5Table::new(#table_name)
          #(#column_calls)*
          #(#option_calls)*
          .build()
      }
    }
  }
}

/// Every FTS5 column is text; the declared marker type only drives the typed
/// `Column<T>` constants in the generated module.
fn column_call(core: &TokenStream, column: &ColumnInput) -> TokenStream {
  let method = if column.flags.unindexed() {
    Ident::new("unindexed_column", Span::call_site())
  } else {
    Ident::new("column", Span::call_site())
  };
  let name = &column.field_name;
  quote! { .#method(#name, #core::column::ColumnType::Text) }
}

fn option_calls(attrs: &Fts5Attrs) -> Vec<TokenStream> {
  let mut calls = Vec::new();
  push_text_call(&mut calls, "tokenize", attrs.tokenize.as_deref());
  push_text_call(&mut calls, "prefix", attrs.prefix.as_deref());
  push_text_call(&mut calls, "content", attrs.content.as_deref());
  push_text_call(&mut calls, "content_rowid", attrs.content_rowid.as_deref());
  push_text_call(&mut calls, "detail", attrs.detail.as_deref());
  if let Some(columnsize) = attrs.columnsize {
    calls.push(quote! { .columnsize(#columnsize) });
  }
  calls
}

fn push_text_call(calls: &mut Vec<TokenStream>, method: &str, value: Option<&str>) {
  if let Some(value) = value {
    let method = Ident::new(method, Span::call_site());
    calls.push(quote! { .#method(#value) });
  }
}
