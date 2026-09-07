//! Input model for `#[derive(Relational)]`.
//!
//! # Public API
//!
//! - [`parse_relational`], [`RelationalInput`], [`ScalarField`]

use syn::DeriveInput;

use crate::parse::relation_parsing::{is_relation_field, parse_relation_attr, RelationInput};

/// Parsed `#[derive(Relational)]` struct.
pub struct RelationalInput {
  pub struct_name: syn::Ident,
  pub table_name: String,
  pub scalar_fields: Vec<ScalarField>,
  pub relations: Vec<RelationInput>,
}

/// Non-relation field.
pub struct ScalarField {
  pub name: String,
  pub ty: syn::Type,
}

/// Parse derive input into [`RelationalInput`].
pub fn parse_relational(input: &DeriveInput) -> syn::Result<RelationalInput> {
  let struct_name = input.ident.clone();
  let table_name = extract_relational_table(input)?;

  let syn::Data::Struct(data_struct) = &input.data else {
    return Err(syn::Error::new_spanned(
      input,
      "#[derive(Relational)] only works on structs",
    ));
  };
  let syn::Fields::Named(fields) = &data_struct.fields else {
    return Err(syn::Error::new_spanned(
      input,
      "#[derive(Relational)] requires named fields",
    ));
  };

  let mut scalar_fields = Vec::new();
  let mut relations = Vec::new();

  for field in &fields.named {
    if is_relation_field(field) {
      if let Some(rel) = parse_relation_attr(field)? {
        relations.push(rel);
      }
    } else {
      let field_name = field
        .ident
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(field, "field must be named"))?
        .to_string();
      scalar_fields.push(ScalarField {
        name: field_name,
        ty: field.ty.clone(),
      });
    }
  }

  Ok(RelationalInput {
    struct_name,
    table_name,
    scalar_fields,
    relations,
  })
}

fn extract_relational_table(input: &DeriveInput) -> syn::Result<String> {
  for attr in &input.attrs {
    if attr.path().is_ident("relational") {
      let mut table = None::<String>;
      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("table") {
          let v: syn::LitStr = meta.value()?.parse()?;
          table = Some(v.value());
          Ok(())
        } else {
          Err(meta.error("unknown item; expected `table = \"...\"`"))
        }
      })?;
      return table
        .ok_or_else(|| syn::Error::new_spanned(attr, "missing `table` in #[relational(...)]"));
    }
  }
  Err(syn::Error::new_spanned(
    input,
    "#[derive(Relational)] requires #[relational(table = \"...\")] (the `#[table(...)]` attribute is reserved for #[table] schema macro)",
  ))
}
