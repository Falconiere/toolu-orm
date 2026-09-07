//! FromRow derive macro parsing and code generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr};

struct FieldInfo {
  name: syn::Ident,
  name_str: String,
  idx_usize: usize,
  with_fn: Option<String>,
  field_ty: syn::Type,
}

fn parse_fields(input: &DeriveInput) -> syn::Result<Vec<FieldInfo>> {
  let Data::Struct(data) = &input.data else {
    return Err(syn::Error::new_spanned(
      input,
      "FromRow can only be derived for structs",
    ));
  };
  let Fields::Named(fields) = &data.fields else {
    return Err(syn::Error::new_spanned(
      input,
      "FromRow requires named fields",
    ));
  };

  let mut result = Vec::new();
  for (idx, field) in fields.named.iter().enumerate() {
    let idx_usize = idx;
    let name = field
      .ident
      .clone()
      .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))?;
    let name_str = name.to_string();
    let with_fn = extract_with_attr(field)?;
    let field_ty = field.ty.clone();
    result.push(FieldInfo {
      name,
      name_str,
      idx_usize,
      with_fn,
      field_ty,
    });
  }
  Ok(result)
}

fn extract_with_attr(field: &syn::Field) -> syn::Result<Option<String>> {
  for attr in &field.attrs {
    if !attr.path().is_ident("from_row") {
      continue;
    }
    let mut found = None;
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("with") {
        let value = meta.value()?;
        let lit: LitStr = value.parse()?;
        found = Some(lit.value());
        Ok(())
      } else {
        Err(meta.error("unknown from_row attribute"))
      }
    })?;
    return Ok(found);
  }
  Ok(None)
}

/// Builds field extraction for `tokio_postgres::Row::try_get`.
fn build_field_extraction_postgres(
  core: &TokenStream,
  info: &FieldInfo,
) -> syn::Result<TokenStream> {
  let name = &info.name;
  let name_str = &info.name_str;
  let idx = info.idx_usize;
  let field_ty = &info.field_ty;
  if let Some(func) = &info.with_fn {
    let func_ident: syn::Ident = syn::parse_str(func)
      .map_err(|e| syn::Error::new_spanned(&info.name, format!("invalid function name: {e}")))?;
    Ok(quote! {
      #name: {
        let raw = row.try_get::<usize, #field_ty>(#idx)
          .map_err(|e| #core::error::DbCoreError::RowMapping(
            format!("column {} ({}): {}", #idx, #name_str, e)
          ))?;
        #func_ident(raw).map_err(|e| #core::error::DbCoreError::RowMapping(
          format!("column {} ({}): {}", #idx, #name_str, e)
        ))?
      }
    })
  } else {
    Ok(quote! {
      #name: row.try_get::<usize, #field_ty>(#idx)
        .map_err(|e| #core::error::DbCoreError::RowMapping(
          format!("column {} ({}): {}", #idx, #name_str, e)
        ))?
    })
  }
}

pub fn expand_from_row(input: &DeriveInput) -> syn::Result<TokenStream> {
  let core = crate::paths::core();
  let name = &input.ident;
  let field_infos = parse_fields(input)?;

  let column_names: Vec<&str> = field_infos.iter().map(|f| f.name_str.as_str()).collect();

  let postgres_extractions: Vec<TokenStream> = field_infos
    .iter()
    .map(|info| build_field_extraction_postgres(&core, info))
    .collect::<syn::Result<_>>()?;

  Ok(crate::from_row_expand::emit_from_row_impl(
    name,
    &column_names,
    &postgres_extractions,
  ))
}
