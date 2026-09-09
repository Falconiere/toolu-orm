//! Finite-float vector literal rendering for pgvector distance expressions.
//!
//! `SelectBuilder::column_expr` and [`crate::expr::OrderBy`] carry SQL text
//! only, so the query vector is embedded as `'[1,0,0]'::vector` — the same
//! constraint that made `pg_fts::ts_rank` dollar-quote its query. Callers pass
//! `&[f32]`, never a free-form string, so quotes cannot appear in the literal.

use crate::error::DbCoreError;

pub(crate) fn invalid(feature: &str, reason: impl Into<String>) -> DbCoreError {
  DbCoreError::PgVectorInvalidArgument {
    feature: feature.to_owned(),
    reason: reason.into(),
  }
}

/// `'[1,0,0.5]'::vector` with every element proven finite.
///
/// # Errors
///
/// [`DbCoreError::PgVectorInvalidArgument`] when any element is NaN or infinite.
pub(crate) fn vector_literal(feature: &str, embedding: &[f32]) -> Result<String, DbCoreError> {
  let mut body = String::from("[");
  for (i, value) in embedding.iter().enumerate() {
    if !value.is_finite() {
      return Err(invalid(
        feature,
        format!("embedding element {i} must be finite (got {value})"),
      ));
    }
    if i > 0 {
      body.push(',');
    }
    // Display of finite f32 never emits quotes or spaces that break the literal.
    body.push_str(&value.to_string());
  }
  body.push(']');
  Ok(format!("'{body}'::vector"))
}
