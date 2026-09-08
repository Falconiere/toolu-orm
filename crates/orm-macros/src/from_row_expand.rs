//! [`FromRow`] derive output: one decoder block per driver, plus the call to
//! `toolu_orm_core::impl_derived_from_row!` that keeps whichever ones this
//! build of `toolu-orm-core` compiled a method for.
//!
//! Every field is read positionally at its own index, with the field's own Rust
//! type, so an `Option<T>` field decodes SQL `NULL` as `None`. The three
//! drivers differ only in how that read is spelled — see [`Driver::read`].

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::from_row::FieldInfo;

/// The three row APIs the derive can decode from.
#[derive(Clone, Copy)]
pub enum Driver {
  Postgres,
  Libsql,
  Rusqlite,
}

impl Driver {
  /// Reads field `info` off a row bound to `row`, yielding the driver's own
  /// `Result<#field_ty, _>`.
  ///
  /// libsql indexes with `i32` while the other two take `usize`, which is the
  /// only reason this can fail: a struct with more than `i32::MAX` fields has
  /// no libsql index to read at.
  fn read(self, row: &Ident, info: &FieldInfo) -> syn::Result<TokenStream> {
    let ty = &info.field_ty;
    let idx = info.idx_usize;
    Ok(match self {
      Self::Postgres => quote! { #row.try_get::<usize, #ty>(#idx) },
      Self::Libsql => {
        let Ok(idx) = i32::try_from(idx) else {
          return Err(syn::Error::new_spanned(
            &info.name,
            "too many fields to index a libsql row",
          ));
        };
        quote! { #row.get::<#ty>(#idx) }
      },
      Self::Rusqlite => quote! { #row.get::<usize, #ty>(#idx) },
    })
  }
}

/// `#name: <read>?` with the driver error mapped to [`DbCoreError::RowMapping`],
/// routed through `#[from_row(with = "…")]` when the field declares one.
fn field_init(
  core: &TokenStream,
  driver: Driver,
  row: &Ident,
  info: &FieldInfo,
) -> syn::Result<TokenStream> {
  let name = &info.name;
  let name_str = &info.name_str;
  let idx = info.idx_usize;
  let read = driver.read(row, info)?;
  let mapping = quote! {
    |e| #core::error::DbCoreError::RowMapping(
      format!("column {} ({}): {}", #idx, #name_str, e)
    )
  };

  let Some(func) = &info.with_fn else {
    return Ok(quote! { #name: #read.map_err(#mapping)? });
  };
  let func_ident: syn::Ident = syn::parse_str(func)
    .map_err(|e| syn::Error::new_spanned(&info.name, format!("invalid function name: {e}")))?;
  Ok(quote! {
    #name: {
      let raw = #read.map_err(#mapping)?;
      #func_ident(raw).map_err(#mapping)?
    }
  })
}

/// One `|row| { Ok(#name { … }) }` decoder for `driver`.
fn decoder(
  core: &TokenStream,
  driver: Driver,
  name: &Ident,
  fields: &[FieldInfo],
) -> syn::Result<TokenStream> {
  // Named per driver so the three decoders cannot shadow one another, and
  // interpolated into the block so `macro_rules!` hygiene resolves it to the
  // binding `impl_derived_from_row!` declares from this very ident.
  let row = format_ident!(
    "row_{}",
    match driver {
      Driver::Postgres => "pg",
      Driver::Libsql => "libsql",
      Driver::Rusqlite => "rusqlite",
    }
  );
  let inits: Vec<TokenStream> = fields
    .iter()
    .map(|info| field_init(core, driver, &row, info))
    .collect::<syn::Result<_>>()?;
  Ok(quote! {
    |#row| { Ok(#name { #(#inits,)* }) }
  })
}

/// The whole derive output: three decoders handed to
/// `impl_derived_from_row!`, which keeps the ones this build has a method for.
pub fn emit_from_row_impl(
  core: &TokenStream,
  name: &Ident,
  column_names: &[&str],
  fields: &[FieldInfo],
) -> syn::Result<TokenStream> {
  let pg = decoder(core, Driver::Postgres, name, fields)?;
  let libsql = decoder(core, Driver::Libsql, name, fields)?;
  let rusqlite = decoder(core, Driver::Rusqlite, name, fields)?;
  Ok(quote! {
    #core::impl_derived_from_row! {
      #name, &[#(#column_names),*],
      postgres = #pg,
      libsql = #libsql,
      rusqlite = #rusqlite,
    }
  })
}
