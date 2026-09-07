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

pub fn expand_columns_module(input: &TableInput) -> TokenStream {
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
      let marker = column_marker_type(col);
      quote! {
        pub const #const_name: toolu_orm_core::query_column::Column<#marker> =
          toolu_orm_core::query_column::Column::new(#table_name, #col_name);
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
fn column_marker_type(col: &ColumnInput) -> TokenStream {
  if col.flags.as_text() {
    return quote! { toolu_orm_core::column::Text };
  }
  if let Some(explicit) = &col.explicit_column_type {
    return simple_marker_type(explicit.as_str());
  }
  match &col.type_spec {
    TypeSpec::Varchar(n) => {
      let n = *n;
      quote! { toolu_orm_core::column::Varchar<#n> }
    },
    TypeSpec::Char(n) => {
      let n = *n;
      quote! { toolu_orm_core::column::Char<#n> }
    },
    TypeSpec::Simple(name) => simple_marker_type(name.as_str()),
  }
}

fn simple_marker_type(name: &str) -> TokenStream {
  match name {
    "Integer" => quote! { toolu_orm_core::column::Integer },
    "Real" => quote! { toolu_orm_core::column::Real },
    "Blob" => quote! { toolu_orm_core::column::Blob },
    "Uuid" => quote! { toolu_orm_core::column::Uuid },
    "Boolean" => quote! { toolu_orm_core::column::Boolean },
    "Timestamp" => quote! { toolu_orm_core::column::Timestamp },
    "Date" => quote! { toolu_orm_core::column::Date },
    "Time" => quote! { toolu_orm_core::column::Time },
    "Json" => quote! { toolu_orm_core::column::Json },
    "BigInt" => quote! { toolu_orm_core::column::BigInt },
    "SmallInt" => quote! { toolu_orm_core::column::SmallInt },
    "Serial" => quote! { toolu_orm_core::column::Serial },
    "BigSerial" => quote! { toolu_orm_core::column::BigSerial },
    "Jsonb" => quote! { toolu_orm_core::column::Jsonb },
    "Numeric" => quote! { toolu_orm_core::column::Numeric },
    // Text and unknown/custom types (enums stored as text) both use Text marker
    _ => quote! { toolu_orm_core::column::Text },
  }
}
