//! The issue #41 query through `SelectBuilder`: distance projection, alias
//! `ORDER BY`, and `LIMIT` — pure SQL, no database.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::OrderBy;
use toolu_orm_core::pgvector::PgVectorOps;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("items", "embedding");

#[test]
fn the_knn_shape_from_the_issue_is_expressible() -> TestResult {
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[0.1, 0.2, 0.3])?;

  let (sql, params) = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .column_expr(dist.sql(), "distance")
    .order_by(OrderBy::alias_asc("distance"))
    .limit(5)
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id", "#,
      r#""items"."embedding" <-> '[0.1,0.2,0.3]'::vector AS "distance""#,
      r#" FROM "items""#,
      r#" ORDER BY "distance" ASC LIMIT $1"#,
    )
  );
  assert_eq!(params, vec![Value::Integer(5)]);
  assert!(!sql.contains("MATCH"));
  assert!(!sql.contains("\"k\""));
  Ok(())
}

#[test]
fn a_query_can_order_by_distance_without_selecting_it() -> TestResult {
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1.0, 0.0])?;

  let (sql, params) = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(dist)
    .limit(3)
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "items""#,
      r#" ORDER BY "items"."embedding" <-> '[1,0]'::vector ASC"#,
      r#" LIMIT $1"#,
    )
  );
  assert_eq!(params, vec![Value::Integer(3)]);
  Ok(())
}

#[test]
fn cosine_and_neg_inner_product_ops_reach_the_builder() -> TestResult {
  let cosine = EMBEDDING.cosine_distance_for(Dialect::Postgres, &[1.0])?;
  let (sql, _) = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(cosine.asc())
    .to_sql_for(Dialect::Postgres);
  assert!(sql.contains("<=>"));

  let nip = EMBEDDING.neg_inner_product_for(Dialect::Postgres, &[1.0])?;
  let (sql, _) = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .order_by(nip.asc())
    .to_sql_for(Dialect::Postgres);
  assert!(sql.contains("<#>"));
  Ok(())
}

#[test]
fn sqlite_distance_is_refused_before_any_sql() -> TestResult {
  let err = EMBEDDING
    .l2_distance_for(Dialect::Sqlite, &[1.0])
    .err()
    .ok_or("must refuse")?;
  let message = err.to_string();
  assert!(message.contains("sqlite"));
  assert!(message.contains("pgvector"));
  Ok(())
}
