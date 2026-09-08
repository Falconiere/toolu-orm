//! Which `vec0` column each field is, and the builder call that renders it.
//!
//! Every refusal here is a compile error spanning the field's type, so a table
//! `vec0` would reject at `CREATE VIRTUAL TABLE` time never reaches a
//! migration file.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::Error;

use toolu_orm_core::vec0::is_vec0_ident;

use crate::parse::{ColumnInput, TypeSpec};

use super::types::{auxiliary_type, key_type, metadata_type};

/// One `Vec0Table` method call per declared field, in declaration order.
pub fn builder_calls(core: &TokenStream, columns: &[ColumnInput]) -> syn::Result<Vec<TokenStream>> {
  columns
    .iter()
    .map(|column| builder_call(core, column))
    .collect()
}

pub fn err(column: &ColumnInput, message: impl Into<String>) -> Error {
  Error::new_spanned(&column.original_type, message.into())
}

fn builder_call(core: &TokenStream, column: &ColumnInput) -> syn::Result<TokenStream> {
  let name = &column.field_name;
  if !is_vec0_ident(name) {
    return Err(err(
      column,
      format!(
        "`{name}` cannot be a vec0 column name: vec0 parses its own arguments and has no \
         quoting, so the name must match [A-Za-z][A-Za-z0-9_]*"
      ),
    ));
  }
  check_unsupported_constraints(column)?;
  if is_vector(column) {
    vector_call(core, column)
  } else {
    scalar_call(core, column)
  }
}

/// `vec0` stores metadata columns strictly typed and unconstrained: only
/// `primary_key` survives into its constructor, so anything else would be
/// parsed here and silently dropped.
fn check_unsupported_constraints(column: &ColumnInput) -> syn::Result<()> {
  let offender = if column.flags.not_null() {
    "not_null"
  } else if column.flags.unique() {
    "unique"
  } else if column.flags.unindexed() {
    "unindexed"
  } else if column.default.is_some() {
    "default"
  } else if column.references.is_some() {
    "references"
  } else {
    return Ok(());
  };
  Err(err(
    column,
    format!(
      "`{offender}` on `{}`: a vec0 column takes no constraints beyond #[column(primary_key)]",
      column.field_name
    ),
  ))
}

fn is_vector(column: &ColumnInput) -> bool {
  matches!(&column.type_spec, TypeSpec::Simple(name) if name == "Vector")
}

fn vector_call(core: &TokenStream, column: &ColumnInput) -> syn::Result<TokenStream> {
  let name = &column.field_name;
  if column.flags.primary_key() || column.flags.partition_key() || column.flags.auxiliary() {
    return Err(err(
      column,
      format!("`{name}` is a vector column, so it is neither a key nor an auxiliary column"),
    ));
  }
  let Some(dim) = column.vec0.dim else {
    return Err(err(
      column,
      format!("`{name}` is a Vector and needs its dimension: #[column(dim = 1024)]"),
    ));
  };
  let element = element_variant(column)?;
  let head = quote! { #name, #core::column::VectorElement::#element, #dim };
  match metric_variant(column)? {
    Some(metric) => Ok(quote! { .vector_metric(#head, #core::vec0::DistanceMetric::#metric) }),
    None => Ok(quote! { .vector(#head) }),
  }
}

fn element_variant(column: &ColumnInput) -> syn::Result<Ident> {
  let variant = match column.vec0.element.as_deref() {
    None | Some("float") => "Float",
    Some("int8") => "Int8",
    Some("bit") => "Bit",
    Some(other) => {
      return Err(err(
        column,
        format!("unknown element `{other}`, expected \"float\", \"int8\" or \"bit\""),
      ))
    },
  };
  Ok(Ident::new(variant, Span::call_site()))
}

fn metric_variant(column: &ColumnInput) -> syn::Result<Option<Ident>> {
  let Some(metric) = column.vec0.distance_metric.as_deref() else {
    return Ok(None);
  };
  if column.vec0.element.as_deref() == Some("bit") {
    return Err(err(
      column,
      format!(
        "`{}` is a bit vector, and a bit vector has no distance_metric",
        column.field_name
      ),
    ));
  }
  let variant = match metric {
    "l2" => "L2",
    "cosine" => "Cosine",
    "l1" => "L1",
    other => {
      return Err(err(
        column,
        format!("unknown distance_metric `{other}`, expected \"l2\", \"cosine\" or \"l1\""),
      ))
    },
  };
  Ok(Some(Ident::new(variant, Span::call_site())))
}

fn scalar_call(core: &TokenStream, column: &ColumnInput) -> syn::Result<TokenStream> {
  let name = &column.field_name;
  if let Some(key) = column.vec0.first_key() {
    return Err(err(
      column,
      format!("`{key}` on `{name}`: only a Vector column carries it"),
    ));
  }
  let roles = usize::from(column.flags.primary_key())
    + usize::from(column.flags.partition_key())
    + usize::from(column.flags.auxiliary());
  if roles > 1 {
    return Err(err(
      column,
      format!("`{name}` is at most one of primary_key, partition_key and auxiliary"),
    ));
  }
  if column.flags.primary_key() {
    let key = key_type(column)?;
    return Ok(quote! { .primary_key(#name, #core::vec0::Vec0KeyType::#key) });
  }
  if column.flags.partition_key() {
    let key = key_type(column)?;
    return Ok(quote! { .partition_key(#name, #core::vec0::Vec0KeyType::#key) });
  }
  if column.flags.auxiliary() {
    let aux = auxiliary_type(column)?;
    return Ok(quote! { .auxiliary(#name, #core::vec0::Vec0AuxiliaryType::#aux) });
  }
  let metadata = metadata_type(column)?;
  Ok(quote! { .metadata(#name, #core::vec0::Vec0MetadataType::#metadata) })
}
