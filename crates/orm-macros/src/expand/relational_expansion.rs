//! Expansion for `#[derive(Relational)]` → `FromRelationalRow`.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::relation_parsing::{RelationInput, RelationInputKind};
use crate::paths;
use crate::relational::RelationalInput;

fn relation_metadata_revision(relations: &[RelationInput]) -> usize {
  relations
    .iter()
    .map(|r| {
      let mut n = r.target_table.len() + r.foreign_key.len();
      if let RelationInputKind::ManyToMany {
        through_table,
        local_key,
      } = &r.kind
      {
        n += through_table.len() + local_key.len();
      }
      n
    })
    .sum()
}

/// Generate `FromRelationalRow` for a relational struct.
pub fn expand_relational(input: &RelationalInput) -> TokenStream {
  let core = paths::core();
  let struct_name = &input.struct_name;
  let _ = relation_metadata_revision(&input.relations);
  let table_lit = syn::LitStr::new(&input.table_name, proc_macro2::Span::call_site());
  let scalar_names: Vec<&str> = input
    .scalar_fields
    .iter()
    .map(|f| f.name.as_str())
    .collect();

  let scalar_extractions: Vec<TokenStream> = input
    .scalar_fields
    .iter()
    .enumerate()
    .map(|(i, field)| {
      let field_ident = syn::Ident::new(&field.name, proc_macro2::Span::call_site());
      let ty = &field.ty;
      quote! {
        let #field_ident: #ty = #core::relational_row::RelationDeserializer::new(values)
          .get(#i)
          .map_err(|e| #core::error::DbCoreError::RowMapping(
            format!("field '{}': {}", stringify!(#field_ident), e)
          ))?;
      }
    })
    .collect();

  let relation_extractions: Vec<TokenStream> = input
    .relations
    .iter()
    .enumerate()
    .map(|(i, rel)| {
      let field_ident = syn::Ident::new(&rel.field_name, proc_macro2::Span::call_site());
      let inner_type = &rel.inner_type;
      let col_index = input.scalar_fields.len() + i;
      let column_lits: Vec<syn::LitStr> = rel
        .columns
        .iter()
        .map(|c| syn::LitStr::new(c, proc_macro2::Span::call_site()))
        .collect();

      match &rel.kind {
        RelationInputKind::HasMany | RelationInputKind::ManyToMany { .. } => {
          quote! {
            let #field_ident = {
              let json_val = &values[#col_index];
              if json_val.is_null() {
                Vec::new()
              } else if let Some(rows) = json_val.as_array() {
                if rows.is_empty() {
                  Vec::new()
                } else {
                  let mut result = Vec::with_capacity(rows.len());
                  for row in rows {
                    let inner: #inner_type = (if let Some(arr) = row.as_array() {
                      #core::relational_row::from_json_object_slice(
                        arr.as_slice(),
                        &[#(#column_lits),*],
                      )
                    } else {
                      #core::serde_json::from_value(row.clone()).map_err(|e| {
                        #core::error::DbCoreError::RowMapping(e.to_string())
                      })
                    })
                    .map_err(|e| #core::error::DbCoreError::RowMapping(
                      format!("field '{}': {}", stringify!(#field_ident), e)
                    ))?;
                    result.push(inner);
                  }
                  result
                }
              } else {
                return Err(#core::error::DbCoreError::RowMapping(
                  format!("field '{}': expected JSON array", stringify!(#field_ident)),
                ));
              }
            };
          }
        },
        RelationInputKind::BelongsTo => {
          quote! {
            let #field_ident = {
              let json_val = &values[#col_index];
              if json_val.is_null() {
                None
              } else if let Some(arr) = json_val.as_array() {
                let inner: #inner_type = #core::relational_row::from_json_object_slice(
                  arr.as_slice(),
                  &[#(#column_lits),*],
                )
                .map_err(|e| #core::error::DbCoreError::RowMapping(
                  format!("field '{}': {}", stringify!(#field_ident), e)
                ))?;
                Some(inner)
              } else {
                let inner: #inner_type = #core::serde_json::from_value(json_val.clone())
                  .map_err(|e| #core::error::DbCoreError::RowMapping(
                    format!("field '{}': {}", stringify!(#field_ident), e)
                  ))?;
                Some(inner)
              }
            };
          }
        },
      }
    })
    .collect();

  let all_field_names: Vec<syn::Ident> = input
    .scalar_fields
    .iter()
    .map(|f| syn::Ident::new(&f.name, proc_macro2::Span::call_site()))
    .chain(
      input
        .relations
        .iter()
        .map(|r| syn::Ident::new(&r.field_name, proc_macro2::Span::call_site())),
    )
    .collect();

  quote! {
    impl #struct_name {
      /// Source table from `#[relational(table = "...")]`.
      pub const RELATIONAL_TABLE: &'static str = #table_lit;
    }

    impl #core::relational_row::FromRelationalRow for #struct_name {
      const SCALAR_COLUMNS: &'static [&'static str] = &[
        #(#scalar_names),*
      ];

      fn from_relational_values(
        values: &[#core::serde_json::Value],
      ) -> Result<Self, #core::error::DbCoreError> {
        #(#scalar_extractions)*
        #(#relation_extractions)*

        Ok(Self {
          #(#all_field_names),*
        })
      }
    }
  }
}
