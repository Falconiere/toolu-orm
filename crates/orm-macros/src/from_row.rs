//! FromRow derive macro parsing and code generation.

use proc_macro2::TokenStream;
use syn::{Data, DeriveInput, Fields, LitStr};

pub struct FieldInfo {
  pub name: syn::Ident,
  pub name_str: String,
  pub idx_usize: usize,
  pub with_fn: Option<String>,
  pub field_ty: syn::Type,
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

pub fn expand_from_row(input: &DeriveInput) -> syn::Result<TokenStream> {
  let core = crate::paths::core();
  let name = &input.ident;
  let field_infos = parse_fields(input)?;

  let column_names: Vec<&str> = field_infos.iter().map(|f| f.name_str.as_str()).collect();

  crate::from_row_expand::emit_from_row_impl(&core, name, &column_names, &field_infos)
}
