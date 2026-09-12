//! The `TableSchema` impl, built through the core `vec0` builder.

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::TableInput;
use crate::paths;

use super::columns::builder_calls;

pub fn expand(input: &TableInput) -> syn::Result<TokenStream> {
  let struct_name = &input.struct_name;
  let table_name = &input.table_name;
  let core = paths::core();
  let calls = builder_calls(&core, &input.columns)?;

  // `build_prevalidated`, not `build`: the table name, every column name and
  // the bit/distance_metric pair were checked above, where the diagnostic can
  // point at the offending field — and `table_def` cannot return a `Result`.
  // Zero or many vector columns are both legal `vec0` shapes, so they are not
  // refused here.
  Ok(quote! {
    impl #core::table::TableSchema for #struct_name {
      fn table_def() -> #core::table::TableDef {
        #core::vec0::Vec0Table::new(#table_name)
          #(#calls)*
          .build_prevalidated()
      }
    }
  })
}
