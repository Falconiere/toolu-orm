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

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::parse::{ColumnInput, IndexInput, TableInput, TypeSpec};
use crate::paths;

pub fn expand(input: &TableInput) -> TokenStream {
  let core = paths::core();
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;
  let strict = input.strict;

  let column_defs = input.columns.iter().map(|c| column_def_tokens(&core, c));
  let index_defs = input.indexes.iter().map(|i| index_def_tokens(&core, i));

  quote! {
      #[automatically_derived]
      impl #core::table::TableSchema for #struct_name {
          fn table_def() -> #core::table::TableDef {
              #core::table::TableDef {
                  name: #table_name.to_owned(),
                  columns: vec![
                      #(#column_defs),*
                  ],
                  indexes: vec![
                      #(#index_defs),*
                  ],
                  strict: #strict,
                  kind: #core::table::TableKind::Ordinary,
              }
          }
      }
  }
}

fn column_def_tokens(core: &TokenStream, col: &ColumnInput) -> TokenStream {
  let name = &col.field_name;
  let mapped = if col.flags.as_text() {
    MappedColumn::Marker(column_type_tokens(core, "Text"))
  } else if let Some(explicit) = &col.explicit_column_type {
    map_type(
      core,
      &TypeSpec::Simple(explicit.clone()),
      &col.original_type,
    )
  } else {
    map_type(core, &col.type_spec, &col.original_type)
  };
  let (col_type, check_expr) = match mapped {
    MappedColumn::Marker(tokens) => (tokens, quote! { None }),
    MappedColumn::Enum { type_path } => {
      let col_name = &col.field_name;
      (
        column_type_tokens(core, "Text"),
        quote! {{
          let variants = <#type_path as #core::column::EnumSchema>::variants();
          let values: Vec<String> = variants.iter().map(|v| format!("'{v}'")).collect();
          Some(format!(r#"CHECK("{}" IN ({}))"#, #col_name, values.join(", ")))
        }},
      )
    },
  };
  let pk = col.flags.primary_key();
  let nn = col.flags.not_null();
  let unique = col.flags.unique();
  let unindexed = col.flags.unindexed();
  let default_expr = option_string_tokens(&col.default);
  let refs_expr = option_string_tokens(&col.references);
  let on_delete_expr = fk_action_tokens(core, &col.on_delete);
  let on_update_expr = fk_action_tokens(core, &col.on_update);

  quote! {
      #core::column::ColumnDef {
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
          unindexed: #unindexed,
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

fn fk_action_tokens(core: &TokenStream, opt: &Option<String>) -> TokenStream {
  let variant = match opt.as_deref() {
    Some("cascade") => "Cascade",
    Some("set_null") => "SetNull",
    Some("set_default") => "SetDefault",
    Some("restrict") => "Restrict",
    Some("no_action") => "NoAction",
    _ => return quote! { None },
  };
  let variant = Ident::new(variant, Span::call_site());
  quote! { Some(#core::column::ForeignKeyAction::#variant) }
}

fn index_def_tokens(core: &TokenStream, idx: &IndexInput) -> TokenStream {
  let name = &idx.name;
  let columns = &idx.columns;
  let unique = idx.unique;
  quote! {
      #core::index::IndexDef {
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

fn column_type_tokens(core: &TokenStream, variant: &str) -> TokenStream {
  let variant = Ident::new(variant, Span::call_site());
  quote! { #core::column::ColumnType::#variant }
}

fn map_type(core: &TokenStream, type_spec: &TypeSpec, original_type: &syn::Type) -> MappedColumn {
  let name = match type_spec {
    TypeSpec::Varchar(n) => {
      let n = *n;
      let varchar = column_type_tokens(core, "Varchar");
      return MappedColumn::Marker(quote! { #varchar(#n) });
    },
    TypeSpec::Char(n) => {
      let n = *n;
      let char_type = column_type_tokens(core, "Char");
      return MappedColumn::Marker(quote! { #char_type(#n) });
    },
    TypeSpec::Simple(name) => name.as_str(),
  };
  match name {
    "Integer" | "Real" | "Blob" | "Uuid" | "Boolean" | "Timestamp" | "Date" | "Time" | "Json"
    | "BigInt" | "SmallInt" | "Text" | "Serial" | "BigSerial" | "Jsonb" | "Numeric" => {
      MappedColumn::Marker(column_type_tokens(core, name))
    },
    // Unknown type: assume it implements EnumSchema
    _ => MappedColumn::Enum {
      type_path: original_type.clone(),
    },
  }
}

pub fn expand_builder_methods(input: &TableInput) -> TokenStream {
  let core = paths::core();
  let query = paths::query();
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;
  let select_doc = format!("A SELECT builder over \"{table_name}\".");
  let select_for_doc =
    format!("A SELECT builder over \"{table_name}\" that selects the columns of `T`.");
  let insert_doc = format!("An INSERT builder over \"{table_name}\".");
  let update_doc = format!("An UPDATE builder over \"{table_name}\".");
  let delete_doc = format!("A DELETE builder over \"{table_name}\".");

  quote! {
    #[automatically_derived]
    impl #struct_name {
      #[doc = #select_doc]
      pub fn select() -> #query::select::SelectBuilder {
        #query::select::SelectBuilder::new(#table_name)
      }
      #[doc = #select_for_doc]
      pub fn select_for<T: #core::row::FromRow>() -> #query::select::SelectBuilder {
        #query::select::SelectBuilder::new(#table_name)
          .columns_raw(T::REQUIRED_COLUMNS)
      }
      #[doc = #insert_doc]
      pub fn insert() -> #query::insert::InsertBuilder {
        #query::insert::InsertBuilder::new(#table_name)
      }
      #[doc = #update_doc]
      pub fn update() -> #query::update::UpdateBuilder {
        #query::update::UpdateBuilder::new(#table_name)
      }
      #[doc = #delete_doc]
      pub fn delete() -> #query::delete::DeleteBuilder {
        #query::delete::DeleteBuilder::new(#table_name)
      }
    }
  }
}
