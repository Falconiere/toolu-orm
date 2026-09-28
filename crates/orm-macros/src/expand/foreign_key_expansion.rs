//! `ForeignKeyDef` literals for a table's `#[foreign_key(...)]` attributes.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::ForeignKeyInput;

use super::schema_expansion::fk_action_tokens;

/// One `ForeignKeyDef` literal, named as the declaration asked or by default.
pub fn foreign_key_tokens(
  core: &TokenStream,
  table_name: &str,
  fk: &ForeignKeyInput,
) -> TokenStream {
  let name = fk.name(table_name);
  let columns = fk.columns.iter().map(ToString::to_string);
  let references_table = &fk.references_table;
  let references_columns = &fk.references_columns;
  let on_delete = fk_action_tokens(core, &fk.on_delete);
  let on_update = fk_action_tokens(core, &fk.on_update);
  quote! {
      #core::snapshot::ForeignKeyDef {
          name: #name.to_owned(),
          columns: vec![#(#columns.to_owned()),*],
          references_table: #references_table.to_owned(),
          references_columns: vec![#(#references_columns.to_owned()),*],
          on_delete: #on_delete,
          on_update: #on_update,
      }
  }
}
