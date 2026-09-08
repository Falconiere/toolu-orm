mod column_enum;
mod expand;
mod from_row;
mod from_row_expand;
mod fts5;
mod parse;
mod paths;
mod relational;
mod vec0;
mod view;

use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{parse_macro_input, ItemStruct, Meta};

#[proc_macro_attribute]
pub fn table(attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut item_struct = parse_macro_input!(item as ItemStruct);

  let (table_name, strict) = match parse::parse_table_attrs(attr) {
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

/// Declares a `CREATE VIRTUAL TABLE … USING fts5(…)` table.
///
/// ```ignore
/// #[fts5_table(name = "memory_fts", tokenize = "porter unicode61")]
/// pub struct MemoryFts {
///   #[column(unindexed)]
///   pub memory_id: Text,
///   pub body: Text,
/// }
/// ```
///
/// Accepts `name` (required), `tokenize`, `prefix`, `content`,
/// `content_rowid`, `columnsize` and `detail`. Indexes are rejected: SQLite
/// cannot index a virtual table.
#[proc_macro_attribute]
pub fn fts5_table(attr: TokenStream, item: TokenStream) -> TokenStream {
  match expand_fts5_table(attr, item) {
    Ok(tokens) => tokens,
    Err(e) => e.to_compile_error().into(),
  }
}

fn expand_fts5_table(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
  let item_struct = syn::parse::<ItemStruct>(item)?;
  let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
  let metas: Vec<Meta> = parser.parse(attr)?.into_iter().collect();
  let attrs = fts5::parse_attrs(&metas)?;

  for struct_attr in &item_struct.attrs {
    if struct_attr.path().is_ident("index") || struct_attr.path().is_ident("unique_index") {
      return Err(syn::Error::new_spanned(
        struct_attr,
        "virtual tables cannot declare indexes; remove it from #[fts5_table]",
      ));
    }
  }

  let columns = parse::parse_struct(&item_struct)?;
  if columns.is_empty() {
    return Err(syn::Error::new_spanned(
      &item_struct,
      "an fts5 table needs at least one column",
    ));
  }

  fts5::check_columns(&columns)?;
  let table_name = attrs.table_name()?;
  let input = parse::TableInput {
    table_name,
    strict: false,
    struct_name: item_struct.ident.clone(),
    columns,
    indexes: Vec::new(),
  };

  let schema_impl = fts5::expand(&attrs, &input);
  let columns_mod = expand::expand_columns_module(&input);
  let builder_methods = expand::expand_builder_methods(&input);
  let clean_struct = parse::strip_column_attrs(item_struct);

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

/// Declares a `CREATE VIRTUAL TABLE … USING vec0(…)` table.
///
/// ```ignore
/// #[vec0_table(name = "memory_vec")]
/// pub struct MemoryVec {
///   #[column(primary_key)]
///   pub memory_id: Text,
///   #[column(dim = 1024, distance_metric = "cosine")]
///   pub embedding: Vector,
///   #[column(partition_key)]
///   pub user_id: Integer,
///   pub label: Text,
///   #[column(auxiliary)]
///   pub contents: Text,
/// }
/// ```
///
/// `name` is the only table-level attribute. A `Vector` field needs
/// `#[column(dim = N)]` and takes `element` (`"float"`, `"int8"`, `"bit"`) and
/// `distance_metric` (`"l2"`, `"cosine"`, `"l1"`); a field with none of
/// `primary_key`, `partition_key` and `auxiliary` is a metadata column.
/// Indexes are rejected: SQLite cannot index a virtual table.
///
/// `vec0` is not built into SQLite. The connection has to load `sqlite-vec`
/// before any migration containing one of these tables runs.
#[proc_macro_attribute]
pub fn vec0_table(attr: TokenStream, item: TokenStream) -> TokenStream {
  match vec0::expand_vec0_table(attr, item) {
    Ok(tokens) => tokens,
    Err(e) => e.to_compile_error().into(),
  }
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
