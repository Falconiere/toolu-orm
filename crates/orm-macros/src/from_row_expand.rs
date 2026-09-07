//! [`FromRow`] derive output: Postgres column mapping plus backend stubs.
//!
//! The generated `FromRow` impl always includes `from_pg_row` (real extraction)
//! plus `from_libsql_row` (stub). The `from_rusqlite_row` stub is only emitted
//! when the `rusqlite` feature is active on `orm-macros`, matching the trait
//! shape that `orm-core` exposes for that feature combination.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn emit_from_row_impl(
  name: &Ident,
  column_names: &[&str],
  postgres_extractions: &[TokenStream],
) -> TokenStream {
  let rusqlite_method = if cfg!(feature = "rusqlite") {
    quote! {
      fn from_rusqlite_row(_: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
        Err(toolu_orm_core::error::DbCoreError::RowMapping(
          concat!(stringify!(#name), " is only decoded from Postgres rows").into(),
        ))
      }
    }
  } else {
    quote! {}
  };

  quote! {
    impl toolu_orm_core::row::FromRow for #name {
      const REQUIRED_COLUMNS: &'static [&'static str] = &[#(#column_names),*];

      fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
        Ok(Self {
          #(#postgres_extractions,)*
        })
      }

      fn from_libsql_row(_: &libsql::Row) -> Result<Self, toolu_orm_core::error::DbCoreError> {
        Err(toolu_orm_core::error::DbCoreError::RowMapping(
          concat!(stringify!(#name), " is only decoded from Postgres rows").into(),
        ))
      }

      #rusqlite_method
    }
  }
}
