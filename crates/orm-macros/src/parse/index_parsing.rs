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

use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, ItemStruct, LitStr, Result, Token};

use super::column_parsing::ColumnInput;

pub struct IndexInput {
  pub name: String,
  pub columns: Vec<String>,
  pub unique: bool,
  pub where_clause: Option<String>,
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
  let args: IndexArgs = attr.parse_args()?;
  Ok(IndexInput {
    name: args.name,
    columns: args.columns,
    unique,
    where_clause: args.where_clause,
  })
}

struct IndexArgs {
  name: String,
  columns: Vec<String>,
  where_clause: Option<String>,
}

impl Parse for IndexArgs {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    if !input.peek(LitStr) {
      return Err(syn::Error::new(
        input.span(),
        "first arg must be index name string",
      ));
    }
    let name: LitStr = input.parse()?;
    let mut columns = Vec::new();
    let mut where_clause = None;

    while !input.is_empty() {
      input.parse::<Token![,]>()?;
      if input.is_empty() {
        break;
      }
      if input.peek(Token![where]) {
        let where_token: Token![where] = input.parse()?;
        input.parse::<Token![=]>()?;
        let predicate: LitStr = input.parse()?;
        if where_clause.is_some() {
          return Err(syn::Error::new_spanned(
            where_token,
            "duplicate where clause on index",
          ));
        }
        where_clause = Some(predicate.value());
      } else if input.peek(Ident) {
        let column: Ident = input.parse()?;
        columns.push(column.to_string());
      } else {
        return Err(syn::Error::new(
          input.span(),
          "expected column identifier or where = \"...\"",
        ));
      }
    }

    Ok(Self {
      name: name.value(),
      columns,
      where_clause,
    })
  }
}
