//! The `#[column(...)]` values only `#[vec0_table]` reads.
//!
//! Grouped rather than spread across [`super::ColumnInput`]: three more
//! optional fields on a struct every `#[table]` column also fills would say
//! nothing about which macro reads them.

use syn::meta::ParseNestedMeta;
use syn::{Lit, Result};

use super::column_flags::ColumnFlags;

/// `dim`, `element` and `distance_metric` as written on a field.
///
/// Kept as the raw literal text; `#[vec0_table]` is the only reader and it
/// resolves them against `vec0`'s own vocabulary, so an unknown value is
/// reported there with the field's span.
#[derive(Default)]
pub struct Vec0ColumnInput {
  pub dim: Option<u32>,
  pub element: Option<String>,
  pub distance_metric: Option<String>,
}

impl Vec0ColumnInput {
  /// The first key present, for the error that names what does not belong on
  /// a non-vector column.
  pub fn first_key(&self) -> Option<&'static str> {
    if self.dim.is_some() {
      Some("dim")
    } else if self.element.is_some() {
      Some("element")
    } else if self.distance_metric.is_some() {
      Some("distance_metric")
    } else {
      None
    }
  }
}

/// Reads the vec0-only keys off one `#[column(...)]` entry.
///
/// Anything else is left alone, matching how `#[column(...)]` has always
/// treated a key it does not know.
pub fn parse_vec0_meta(
  meta: &ParseNestedMeta<'_>,
  vec0: &mut Vec0ColumnInput,
  flags: &mut ColumnFlags,
) -> Result<()> {
  if meta.path.is_ident("partition_key") {
    flags.set_partition_key();
  } else if meta.path.is_ident("auxiliary") {
    flags.set_auxiliary();
  } else if meta.path.is_ident("dim") {
    let lit: Lit = meta.value()?.parse()?;
    let Lit::Int(int_lit) = lit else {
      return Err(meta.error("expected an integer, as in #[column(dim = 1024)]"));
    };
    vec0.dim = Some(int_lit.base10_parse()?);
  } else if meta.path.is_ident("element") {
    vec0.element = Some(string_value(meta)?);
  } else if meta.path.is_ident("distance_metric") {
    vec0.distance_metric = Some(string_value(meta)?);
  }
  Ok(())
}

fn string_value(meta: &ParseNestedMeta<'_>) -> Result<String> {
  let lit: Lit = meta.value()?.parse()?;
  let Lit::Str(s) = lit else {
    return Err(meta.error("expected a string literal"));
  };
  Ok(s.value())
}
