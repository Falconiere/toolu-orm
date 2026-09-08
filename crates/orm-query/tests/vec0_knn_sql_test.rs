//! `SelectBuilder` carrying a vec0 KNN: the whole query from issue #21, and
//! the refusals that keep `k` a top-level scan parameter.

use toolu_orm_core::column::{Text, Vector};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0;
use toolu_orm_query::select::SelectBuilder;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("memory_vec", "embedding");
const VEC_MEMORY_ID: Column<Text> = Column::new("memory_vec", "memory_id");
const MEMORY_ID: Column<Text> = Column::new("memories", "id");
const MEMORY_DELETED_AT: Column<Text> = Column::new("memories", "deleted_at");

#[test]
fn knn_emits_match_and_k_as_top_level_conjuncts() -> TestResult {
  let query = Value::vector(&[1.0f32, 2.0]);
  let (sql, params) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id"])
    .knn_for(Dialect::Sqlite, &EMBEDDING, query.clone(), 10)?
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "memory_id" FROM "memory_vec""#,
      r#" WHERE "memory_vec"."embedding" MATCH ?1 AND "k" = ?2"#
    )
  );
  assert_eq!(params, vec![query, Value::Integer(10)]);
  Ok(())
}

#[test]
fn an_extra_filter_stays_a_sibling_of_match_and_k() -> TestResult {
  let query = Value::vector(&[1.0f32, 2.0]);
  let (sql, params) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id"])
    .knn_for(Dialect::Sqlite, &EMBEDDING, query.clone(), 10)?
    .filter(MEMORY_DELETED_AT.is_null())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "memory_id" FROM "memory_vec""#,
      r#" WHERE "memory_vec"."embedding" MATCH ?1 AND "k" = ?2"#,
      r#" AND "memories"."deleted_at" IS NULL"#
    )
  );
  assert_eq!(params, vec![query, Value::Integer(10)]);
  Ok(())
}

/// The query issue #21 opens with, previously impossible to express: a
/// `distance` projection, a join, `.knn`, an ordinary filter, and
/// `ORDER BY distance`.
#[test]
fn the_knn_query_from_the_issue_is_expressible() -> TestResult {
  let query = Value::vector(&[1.0f32, -2.5, 3.0]);
  let distance = vec0::distance_for(Dialect::Sqlite)?;

  let (sql, params) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id"])
    .column_expr(distance.sql(), "distance")
    .knn_for(Dialect::Sqlite, &EMBEDDING, query.clone(), 50)?
    .join("memories", MEMORY_ID.equals(&VEC_MEMORY_ID))
    .filter(MEMORY_DELETED_AT.is_null())
    .order_by(distance)
    .limit(10)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "memory_id", "distance" AS "distance""#,
      r#" FROM "memory_vec""#,
      r#" INNER JOIN "memories" ON "memories"."id" = "memory_vec"."memory_id""#,
      r#" WHERE "memory_vec"."embedding" MATCH ?1 AND "k" = ?2"#,
      r#" AND "memories"."deleted_at" IS NULL"#,
      r#" ORDER BY "distance" ASC LIMIT ?3"#
    )
  );
  assert_eq!(params, vec![query, Value::Integer(50), Value::Integer(10)]);
  Ok(())
}

#[test]
fn columns_raw_can_select_distance_without_an_alias() -> TestResult {
  let (sql, _) = SelectBuilder::new("memory_vec")
    .columns_raw(&["memory_id", "distance"])
    .knn_for(Dialect::Sqlite, &EMBEDDING, Value::vector(&[1.0f32]), 5)?
    .order_by(vec0::distance_for(Dialect::Sqlite)?)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "memory_id", "distance" FROM "memory_vec""#,
      r#" WHERE "memory_vec"."embedding" MATCH ?1 AND "k" = ?2"#,
      r#" ORDER BY "distance" ASC"#
    )
  );
  Ok(())
}

#[test]
fn postgres_knn_is_refused() -> TestResult {
  let error = SelectBuilder::new("memory_vec")
    .knn_for(Dialect::Postgres, &EMBEDDING, Value::vector(&[1.0f32]), 10)
    .err()
    .ok_or("knn must refuse Dialect::Postgres")?;
  let message = error.to_string();
  let DbCoreError::Vec0UnsupportedDialect { feature, dialect } = error else {
    return Err(format!("expected Vec0UnsupportedDialect, got: {error:?}").into());
  };
  assert_eq!(feature, "MATCH");
  assert_eq!(dialect, "postgres");
  assert!(message.contains("postgres"));
  Ok(())
}

#[test]
fn non_positive_k_is_refused_by_knn() -> TestResult {
  let error = SelectBuilder::new("memory_vec")
    .knn_for(Dialect::Sqlite, &EMBEDDING, Value::vector(&[1.0f32]), 0)
    .err()
    .ok_or("k = 0 must be refused")?;
  assert!(matches!(error, DbCoreError::Vec0InvalidArgument { .. }));
  Ok(())
}

#[test]
fn a_second_knn_on_the_same_builder_is_refused() -> TestResult {
  let builder = SelectBuilder::new("memory_vec").knn_for(
    Dialect::Sqlite,
    &EMBEDDING,
    Value::vector(&[1.0f32]),
    10,
  )?;
  let error = builder
    .knn_for(Dialect::Sqlite, &EMBEDDING, Value::vector(&[1.0f32]), 5)
    .err()
    .ok_or("a second knn must be refused")?;
  let DbCoreError::Vec0InvalidArgument { feature, reason } = error else {
    return Err(format!("expected Vec0InvalidArgument, got: {error:?}").into());
  };
  assert_eq!(feature, "knn");
  assert!(reason.contains("already"), "{reason}");
  Ok(())
}

#[test]
fn the_short_knn_agrees_with_the_current_dialect() {
  let short = SelectBuilder::new("memory_vec").knn(&EMBEDDING, Value::vector(&[1.0f32]), 10);
  let explicit = SelectBuilder::new("memory_vec").knn_for(
    Dialect::CURRENT,
    &EMBEDDING,
    Value::vector(&[1.0f32]),
    10,
  );
  assert_eq!(short.is_ok(), explicit.is_ok());
}
