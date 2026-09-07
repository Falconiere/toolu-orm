//! ColumnEnum derive macro for generating EnumSchema with CHECK constraint variants.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

pub fn expand_column_enum(input: &DeriveInput) -> Result<TokenStream> {
  let name = &input.ident;

  let Data::Enum(data_enum) = &input.data else {
    return Err(Error::new_spanned(
      input,
      "ColumnEnum can only be derived for enums",
    ));
  };

  let rename_all = extract_serde_rename_all(&input.attrs);

  let variants: Vec<String> = data_enum
    .variants
    .iter()
    .map(|v| {
      if !matches!(&v.fields, Fields::Unit) {
        return Err(Error::new_spanned(
          v,
          "ColumnEnum variants must be unit variants",
        ));
      }
      let variant_name = v.ident.to_string();
      Ok(apply_rename(&variant_name, rename_all.as_deref()))
    })
    .collect::<Result<Vec<_>>>()?;

  let variant_strs = variants.iter().map(|v| v.as_str());

  Ok(quote! {
    impl toolu_orm_core::column::EnumSchema for #name {
      fn variants() -> &'static [&'static str] {
        &[#(#variant_strs),*]
      }
    }
  })
}

fn extract_serde_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
  for attr in attrs {
    if !attr.path().is_ident("serde") {
      continue;
    }
    let mut rename_all = None;
    let _ = attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("rename_all") {
        let value = meta.value()?;
        let lit: syn::Lit = value.parse()?;
        if let syn::Lit::Str(s) = lit {
          rename_all = Some(s.value());
        }
      }
      Ok(())
    });
    if rename_all.is_some() {
      return rename_all;
    }
  }
  None
}

fn apply_rename(name: &str, rename_all: Option<&str>) -> String {
  match rename_all {
    Some("snake_case") => to_snake_case(name),
    Some("UPPERCASE") => name.to_uppercase(),
    Some("SCREAMING_SNAKE_CASE") => to_snake_case(name).to_uppercase(),
    Some("kebab-case") => to_snake_case(name).replace('_', "-"),
    // "lowercase" and default both lowercase
    _ => name.to_lowercase(),
  }
}

fn to_snake_case(name: &str) -> String {
  let mut result = String::new();
  for (i, ch) in name.chars().enumerate() {
    if ch.is_uppercase() && i > 0 {
      result.push('_');
    }
    result.push(ch.to_lowercase().next().unwrap_or(ch));
  }
  result
}
