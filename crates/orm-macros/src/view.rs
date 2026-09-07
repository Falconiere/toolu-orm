//! View derive macro for generating subset structs from table definitions.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parse;
use syn::{Attribute, Ident, Result};

use crate::parse::{ColumnInput, TypeSpec};

pub struct ViewInput {
  pub struct_name: Ident,
  pub mode: ViewMode,
}

pub enum ViewMode {
  Omit(Vec<String>),
  Pick(Vec<String>),
}

pub fn parse_view_attr(attr: &Attribute) -> Result<ViewInput> {
  attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
    let struct_name: Ident = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let mode_ident: Ident = input.parse()?;
    let content;
    syn::parenthesized!(content in input);
    let fields = content.parse_terminated(Ident::parse, syn::Token![,])?;
    let field_names: Vec<String> = fields.iter().map(|i: &Ident| i.to_string()).collect();
    let mode = match mode_ident.to_string().as_str() {
      "omit" => ViewMode::Omit(field_names),
      "pick" => ViewMode::Pick(field_names),
      _ => {
        return Err(syn::Error::new_spanned(
          &mode_ident,
          "expected `omit` or `pick`",
        ));
      },
    };
    Ok(ViewInput { struct_name, mode })
  })
}

fn map_type_to_rust(core: &TokenStream, type_spec: &TypeSpec) -> TokenStream {
  let type_name = match type_spec {
    TypeSpec::Varchar(_) => "Varchar",
    TypeSpec::Char(_) => "Char",
    TypeSpec::Simple(name) => name.as_str(),
  };
  match type_name {
    "Integer" | "BigInt" | "Timestamp" => quote! { i64 },
    "SmallInt" => quote! { i16 },
    "Real" => quote! { f64 },
    "Boolean" => quote! { bool },
    "Json" => quote! { #core::serde_json::Value },
    "Blob" => quote! { Vec<u8> },
    // Uuid, Text, Date, Time, Varchar, and enum types all map to String
    _ => quote! { String },
  }
}

pub fn generate_view_struct(
  view: &ViewInput,
  columns: &[ColumnInput],
  vis: &syn::Visibility,
) -> TokenStream {
  let core = crate::paths::core();
  let serde = crate::paths::core_serde_literal();
  let struct_name = &view.struct_name;

  let filtered: Vec<&ColumnInput> = columns
    .iter()
    .filter(|c| match &view.mode {
      ViewMode::Omit(omitted) => !omitted.contains(&c.field_name),
      ViewMode::Pick(picked) => picked.contains(&c.field_name),
    })
    .collect();

  let field_tokens: Vec<TokenStream> = filtered
    .iter()
    .map(|col| {
      let field_ident = format_ident!("{}", col.field_name);
      let base_type = map_type_to_rust(&core, &col.type_spec);
      let required = col.flags.primary_key() || col.flags.not_null();
      if required {
        quote! { pub #field_ident: #base_type }
      } else {
        quote! { pub #field_ident: Option<#base_type> }
      }
    })
    .collect();

  // `serde(crate = ...)` points serde's own `extern crate serde as _serde` at
  // the re-export, so a consumer that only depends on the facade still compiles.
  quote! {
    #[derive(Debug, Clone, #core::serde::Serialize, #core::serde::Deserialize)]
    #[serde(crate = #serde)]
    #vis struct #struct_name {
      #(#field_tokens,)*
    }
  }
}
