//! Rendering of the three KNN pieces: vector `MATCH`, hidden `k`, and
//! synthesised `distance`.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, Vec0Ops};
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("memory_vec", "embedding");

#[test]
fn vector_match_qualifies_the_column_and_binds_the_blob() -> TestResult {
  let query = Value::vector(&[1.0f32, 2.0]);
  let expr = EMBEDDING.matches_for(Dialect::Sqlite, query.clone())?;
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#""memory_vec"."embedding" MATCH ?1"#);
  assert_eq!(params, vec![query]);
  Ok(())
}

#[test]
fn vector_match_continues_the_callers_parameter_numbering() -> TestResult {
  let expr = EMBEDDING.matches_for(Dialect::Sqlite, Value::vector(&[1.0f32]))?;
  let (sql, params) = expr.to_sql_fragment_for(3, Dialect::Sqlite);

  assert_eq!(sql, r#""memory_vec"."embedding" MATCH ?3"#);
  assert_eq!(params.len(), 1);
  Ok(())
}

#[test]
fn k_eq_renders_the_quoted_hidden_column() -> TestResult {
  let expr = vec0::k_eq_for(Dialect::Sqlite, 10)?;
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);

  assert_eq!(sql, r#""k" = ?1"#);
  assert_eq!(params, vec![Value::Integer(10)]);
  Ok(())
}

#[test]
fn distance_renders_as_a_quoted_identifier() -> TestResult {
  let distance = vec0::distance_for(Dialect::Sqlite)?;
  assert_eq!(distance.sql(), r#""distance""#);
  assert_eq!(distance.asc().to_sql(), r#""distance" ASC"#);
  assert_eq!(distance.desc().to_sql(), r#""distance" DESC"#);
  Ok(())
}

#[test]
fn distance_converts_to_ascending_order_by_default() -> TestResult {
  let order: toolu_orm_core::expr::OrderBy = vec0::distance_for(Dialect::Sqlite)?.into();
  assert_eq!(order.to_sql(), r#""distance" ASC"#);
  Ok(())
}

/// Short forms follow `Dialect::CURRENT` — Sqlite on the default lane,
/// Postgres on the postgres lane. Agreement with the explicit call is the
/// only assertion that holds on both.
#[test]
fn the_short_forms_agree_with_the_current_dialect() {
  assert_eq!(
    EMBEDDING.matches(Value::vector(&[1.0f32])).is_ok(),
    EMBEDDING
      .matches_for(Dialect::CURRENT, Value::vector(&[1.0f32]))
      .is_ok()
  );
  assert_eq!(
    vec0::k_eq(10).is_ok(),
    vec0::k_eq_for(Dialect::CURRENT, 10).is_ok()
  );
  assert_eq!(
    vec0::distance().is_ok(),
    vec0::distance_for(Dialect::CURRENT).is_ok()
  );
}
