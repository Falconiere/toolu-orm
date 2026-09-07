//! [`FromRow`] derive output: Postgres column mapping plus backend stubs.
//!
//! The generated `FromRow` impl always includes `from_pg_row` (real extraction)
//! plus `from_libsql_row` (stub). The `from_rusqlite_row` stub is only emitted
//! when the `rusqlite` feature is active on `orm-macros`, matching the trait
//! shape that `orm-core` exposes for that feature combination.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::paths;

pub fn emit_from_row_impl(
  name: &Ident,
  column_names: &[&str],
  postgres_extractions: &[TokenStream],
) -> TokenStream {
  let core = paths::core();
  let rusqlite_method = if cfg!(feature = "rusqlite") {
    quote! {
      fn from_rusqlite_row(_: &#core::rusqlite::Row<'_>) -> Result<Self, #core::error::DbCoreError> {
        Err(#core::error::DbCoreError::RowMapping(
          concat!(stringify!(#name), " is only decoded from Postgres rows").into(),
        ))
      }
    }
  } else {
    quote! {}
  };

  quote! {
    impl #core::row::FromRow for #name {
      const REQUIRED_COLUMNS: &'static [&'static str] = &[#(#column_names),*];

      fn from_pg_row(row: &#core::tokio_postgres::Row) -> Result<Self, #core::error::DbCoreError> {
        Ok(Self {
          #(#postgres_extractions,)*
        })
      }

      fn from_libsql_row(_: &#core::libsql::Row) -> Result<Self, #core::error::DbCoreError> {
        Err(#core::error::DbCoreError::RowMapping(
          concat!(stringify!(#name), " is only decoded from Postgres rows").into(),
        ))
      }

      #rusqlite_method
    }
  }
}
