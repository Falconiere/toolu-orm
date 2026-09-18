//! `CASE` rendering and scalar `ORDER BY` terms.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{Column, NumericOps};
use toolu_orm_core::value::Value;

const HITS: Column<Integer> = Column::new("memories", "access_count");
const LAST_SEEN: Column<Text> = Column::new("memories", "last_accessed");
const CREATED_AT: Column<Text> = Column::new("memories", "created_at");

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_two_branch_case_with_else_numbers_predicate_then_value() {
  let label = Scalar::case_when(HITS.gt(10), Scalar::bind("hot"))
    .when(HITS.gt(0), Scalar::bind("warm"))
    .otherwise(Scalar::bind("cold"));

  let (sql, params) = label.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"CASE WHEN "memories"."access_count" > ?1 THEN ?2 WHEN "memories"."access_count" > ?3 THEN ?4 ELSE ?5 END"#
  );
  assert_eq!(
    params,
    vec![
      Value::Integer(10),
      Value::Text("hot".to_owned()),
      Value::Integer(0),
      Value::Text("warm".to_owned()),
      Value::Text("cold".to_owned()),
    ]
  );
}

#[test]
fn a_case_without_else_ends_after_its_last_branch() {
  let (sql, _) = Scalar::case_when(HITS.gt(0), Scalar::bind(1))
    .end()
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"CASE WHEN "memories"."access_count" > $3 THEN $4 END"#
  );
}

#[test]
fn a_scalar_order_term_renders_its_direction_and_binds() -> TestResult {
  let coalesced = Scalar::func(
    "coalesce",
    vec![Scalar::col(&LAST_SEEN), Scalar::col(&CREATED_AT)],
  )?;
  let order = coalesced.desc();

  assert_eq!(
    order.to_sql(),
    r#"coalesce("memories"."last_accessed", "memories"."created_at") DESC"#
  );

  let bound = Scalar::case_when(HITS.gt(0), Scalar::bind(0))
    .otherwise(Scalar::bind(1))
    .asc();
  let (sql, params) = bound.to_sql_fragment_for(2, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"CASE WHEN "memories"."access_count" > ?2 THEN ?3 ELSE ?4 END ASC"#
  );
  assert_eq!(params.len(), 3);
  Ok(())
}

#[test]
fn a_column_order_term_still_renders_exactly_as_before() {
  assert_eq!(HITS.asc().to_sql(), r#""memories"."access_count" ASC"#);
  assert_eq!(HITS.desc().to_sql(), r#""memories"."access_count" DESC"#);
  assert!(HITS
    .asc()
    .to_sql_fragment_for(1, Dialect::Postgres)
    .1
    .is_empty());
}
