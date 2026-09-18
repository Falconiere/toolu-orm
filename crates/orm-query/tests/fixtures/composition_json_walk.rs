//! The production anchor from the issue: seeds expanded out of a bound JSON
//! array by `json_each`, which is SQLite-only.
//!
//! The step and the outer read are the shared, dialect-neutral ones — only the
//! anchor differs, which is exactly the portability boundary.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use super::queries::walk_rows_from;
use super::seed::SEED_VALUE;

/// The issue's `WITH RECURSIVE walk(...) AS (SELECT 'memory', value, 0 FROM
/// json_each(:seeds) UNION ...)`, with `seeds` bound.
///
/// # Errors
///
/// [`DbCoreError::InvalidTableFunction`] can only fire if `json_each` stops
/// being a bare identifier.
pub fn json_seeded_walk(seeds_json: &str, max_depth: i64) -> Result<SelectBuilder, DbCoreError> {
  let seeds =
    TableRef::function("json_each", vec![Value::Text(seeds_json.to_owned())])?.with_alias("seeds");
  let anchor = SelectBuilder::from_table(&seeds)
    .column_scalar(Scalar::sql("'memory'"), "kind")
    .column_as(&seeds.column(&SEED_VALUE), "id")
    .column_scalar(Scalar::sql("CAST(0 AS BIGINT)"), "depth");
  Ok(walk_rows_from(anchor, max_depth))
}
