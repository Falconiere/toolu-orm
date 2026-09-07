//! Expansion of `TableSchema` impl and builder factory methods.
//!
//! # Public API
//!
//! - [`expand`] — generate `TableSchema` impl with column and index defs
//! - [`expand_builder_methods`] — generate select/insert/update/delete methods
//!
//! # Usage
//!
//! ```ignore
//! let tokens = expand(&table_input);
//! let builder = expand_builder_methods(&table_input);
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::{ColumnInput, IndexInput, TableInput, TypeSpec};

pub fn expand(input: &TableInput) -> TokenStream {
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;
  let strict = input.strict;

  let column_defs = input.columns.iter().map(column_def_tokens);
  let index_defs = input.indexes.iter().map(index_def_tokens);

  quote! {
      impl toolu_orm_core::table::TableSchema for #struct_name {
          fn table_def() -> toolu_orm_core::table::TableDef {
              toolu_orm_core::table::TableDef {
                  name: #table_name.to_owned(),
                  columns: vec![
                      #(#column_defs),*
                  ],
                  indexes: vec![
                      #(#index_defs),*
                  ],
                  strict: #strict,
              }
          }
      }
  }
}

fn column_def_tokens(col: &ColumnInput) -> TokenStream {
  let name = &col.field_name;
  let mapped = if col.flags.as_text() {
    MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Text })
  } else if let Some(explicit) = &col.explicit_column_type {
    map_type(&TypeSpec::Simple(explicit.clone()), &col.original_type)
  } else {
    map_type(&col.type_spec, &col.original_type)
  };
  let (col_type, check_expr) = match mapped {
    MappedColumn::Marker(tokens) => (tokens, quote! { None }),
    MappedColumn::Enum { type_path } => {
      let col_name = &col.field_name;
      (
        quote! { toolu_orm_core::column::ColumnType::Text },
        quote! {{
          let variants = <#type_path as toolu_orm_core::column::EnumSchema>::variants();
          let values: Vec<String> = variants.iter().map(|v| format!("'{v}'")).collect();
          Some(format!(r#"CHECK("{}" IN ({}))"#, #col_name, values.join(", ")))
        }},
      )
    },
  };
  let pk = col.flags.primary_key();
  let nn = col.flags.not_null();
  let unique = col.flags.unique();
  let default_expr = option_string_tokens(&col.default);
  let refs_expr = option_string_tokens(&col.references);
  let on_delete_expr = fk_action_tokens(&col.on_delete);
  let on_update_expr = fk_action_tokens(&col.on_update);

  quote! {
      toolu_orm_core::column::ColumnDef {
          name: #name.to_owned(),
          column_type: #col_type,
          primary_key: #pk,
          not_null: #nn,
          default: #default_expr,
          unique: #unique,
          references: #refs_expr,
          on_delete: #on_delete_expr,
          on_update: #on_update_expr,
          check: #check_expr,
      }
  }
}

fn option_string_tokens(opt: &Option<String>) -> TokenStream {
  if let Some(val) = opt {
    quote! { Some(#val.to_owned()) }
  } else {
    quote! { None }
  }
}

fn fk_action_tokens(opt: &Option<String>) -> TokenStream {
  match opt.as_deref() {
    Some("cascade") => {
      quote! { Some(toolu_orm_core::column::ForeignKeyAction::Cascade) }
    },
    Some("set_null") => {
      quote! { Some(toolu_orm_core::column::ForeignKeyAction::SetNull) }
    },
    Some("set_default") => {
      quote! { Some(toolu_orm_core::column::ForeignKeyAction::SetDefault) }
    },
    Some("restrict") => {
      quote! { Some(toolu_orm_core::column::ForeignKeyAction::Restrict) }
    },
    Some("no_action") => {
      quote! { Some(toolu_orm_core::column::ForeignKeyAction::NoAction) }
    },
    _ => quote! { None },
  }
}

fn index_def_tokens(idx: &IndexInput) -> TokenStream {
  let name = &idx.name;
  let columns = &idx.columns;
  let unique = idx.unique;
  quote! {
      toolu_orm_core::index::IndexDef {
          name: #name.to_owned(),
          columns: vec![#(#columns.to_owned()),*],
          unique: #unique,
      }
  }
}

enum MappedColumn {
  Marker(TokenStream),
  Enum { type_path: syn::Type },
}

fn map_type(type_spec: &TypeSpec, original_type: &syn::Type) -> MappedColumn {
  match type_spec {
    TypeSpec::Varchar(n) => {
      let n = *n;
      MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Varchar(#n) })
    },
    TypeSpec::Char(n) => {
      let n = *n;
      MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Char(#n) })
    },
    TypeSpec::Simple(name) => match name.as_str() {
      "Integer" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Integer }),
      "Real" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Real }),
      "Blob" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Blob }),
      "Uuid" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Uuid }),
      "Boolean" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Boolean }),
      "Timestamp" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Timestamp }),
      "Date" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Date }),
      "Time" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Time }),
      "Json" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Json }),
      "BigInt" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::BigInt }),
      "SmallInt" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::SmallInt }),
      "Text" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Text }),
      "Serial" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Serial }),
      "BigSerial" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::BigSerial }),
      "Jsonb" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Jsonb }),
      "Numeric" => MappedColumn::Marker(quote! { toolu_orm_core::column::ColumnType::Numeric }),
      // Unknown type: assume it implements EnumSchema
      _ => MappedColumn::Enum {
        type_path: original_type.clone(),
      },
    },
  }
}

pub fn expand_builder_methods(input: &TableInput) -> TokenStream {
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;

  quote! {
    impl #struct_name {
      pub fn select() -> toolu_orm_query::select::SelectBuilder {
        toolu_orm_query::select::SelectBuilder::new(#table_name)
      }
      pub fn select_for<T: toolu_orm_core::row::FromRow>() -> toolu_orm_query::select::SelectBuilder {
        toolu_orm_query::select::SelectBuilder::new(#table_name)
          .columns_raw(T::REQUIRED_COLUMNS)
      }
      pub fn insert() -> toolu_orm_query::insert::InsertBuilder {
        toolu_orm_query::insert::InsertBuilder::new(#table_name)
      }
      pub fn update() -> toolu_orm_query::update::UpdateBuilder {
        toolu_orm_query::update::UpdateBuilder::new(#table_name)
      }
      pub fn delete() -> toolu_orm_query::delete::DeleteBuilder {
        toolu_orm_query::delete::DeleteBuilder::new(#table_name)
      }
    }
  }
}
