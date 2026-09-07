//! Expansion of the companion columns module with typed column constants.
//!
//! # Public API
//!
//! - [`expand_columns_module`] — generate a `mod` with `Column<T>` constants
//!
//! # Usage
//!
//! ```ignore
//! let tokens = expand_columns_module(&table_input);
//! ```

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};

use crate::parse::{ColumnInput, TableInput, TypeSpec};
use crate::paths;

pub fn expand_columns_module(input: &TableInput) -> TokenStream {
  let core = paths::core();
  let table_name = &input.table_name;
  let mod_name = Ident::new(table_name, Span::call_site());

  let col_names: Vec<&str> = input
    .columns
    .iter()
    .map(|c| c.field_name.as_str())
    .collect();

  let col_consts: Vec<TokenStream> = input
    .columns
    .iter()
    .map(|col| {
      let const_name = Ident::new(&col.field_name, Span::call_site());
      let col_name = &col.field_name;
      let marker = column_marker_type(&core, col);
      quote! {
        pub const #const_name: #core::query_column::Column<#marker> =
          #core::query_column::Column::new(#table_name, #col_name);
      }
    })
    .collect();

  // Lowercase column constants require the non_upper_case_globals lint to be suppressed.
  // Attribute is constructed via interpolated idents (generated code, not hand-written).
  let allow_ident = format_ident!("allow");
  let lint_ident = format_ident!("non_upper_case_globals");

  quote! {
    #[#allow_ident(#lint_ident)]
    pub mod #mod_name {
      pub const TABLE: &str = #table_name;
      pub const ALL_COLUMNS: &[&str] = &[#(#col_names),*];

      #(#col_consts)*
    }
  }
}

/// Maps a column's type spec and flags to the Column<T> generic parameter tokens.
fn column_marker_type(core: &TokenStream, col: &ColumnInput) -> TokenStream {
  if col.flags.as_text() {
    return quote! { #core::column::Text };
  }
  if let Some(explicit) = &col.explicit_column_type {
    return simple_marker_type(core, explicit.as_str());
  }
  match &col.type_spec {
    TypeSpec::Varchar(n) => {
      let n = *n;
      quote! { #core::column::Varchar<#n> }
    },
    TypeSpec::Char(n) => {
      let n = *n;
      quote! { #core::column::Char<#n> }
    },
    TypeSpec::Simple(name) => simple_marker_type(core, name.as_str()),
  }
}

fn simple_marker_type(core: &TokenStream, name: &str) -> TokenStream {
  let marker = Ident::new(marker_name(name), Span::call_site());
  quote! { #core::column::#marker }
}

/// Column marker type name for a type spec. Text is the fallback: unknown and
/// custom types are enums stored as text.
fn marker_name(name: &str) -> &'static str {
  match name {
    "Integer" => "Integer",
    "Real" => "Real",
    "Blob" => "Blob",
    "Uuid" => "Uuid",
    "Boolean" => "Boolean",
    "Timestamp" => "Timestamp",
    "Date" => "Date",
    "Time" => "Time",
    "Json" => "Json",
    "BigInt" => "BigInt",
    "SmallInt" => "SmallInt",
    "Serial" => "Serial",
    "BigSerial" => "BigSerial",
    "Jsonb" => "Jsonb",
    "Numeric" => "Numeric",
    _ => "Text",
  }
}
