//! Scalar-to-scalar comparisons and `LIKE … ESCAPE`.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{like_pattern_literal, Expr, Scalar};
use toolu_orm_core::query_column::{Column, TextOps};
use toolu_orm_core::value::Value;

const BODY: Column<Text> = Column::new("memories", "body");
const CREATED_AT: Column<Text> = Column::new("memories", "created_at");
const HITS: Column<Integer> = Column::new("memories", "access_count");

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_comparison_operator_renders_between_two_scalars() {
  let cases = [
    (Scalar::col(&HITS).eq(Scalar::bind(1)), "="),
    (Scalar::col(&HITS).ne(Scalar::bind(1)), "!="),
    (Scalar::col(&HITS).gt(Scalar::bind(1)), ">"),
    (Scalar::col(&HITS).gte(Scalar::bind(1)), ">="),
    (Scalar::col(&HITS).lt(Scalar::bind(1)), "<"),
    (Scalar::col(&HITS).lte(Scalar::bind(1)), "<="),
  ];
  for (expr, op) in cases {
    let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);
    assert_eq!(sql, format!(r#""memories"."access_count" {op} ?1"#));
    assert_eq!(params, vec![Value::Integer(1)]);
  }
}

#[test]
fn a_call_on_each_side_binds_only_the_right_hand_value() -> TestResult {
  let stamp = Scalar::func("datetime", vec![Scalar::col(&CREATED_AT)])?;
  let cutoff = Scalar::func("datetime", vec![Scalar::bind("2026-09-18T10:00:00Z")])?;

  let (sql, params) = stamp.gte(cutoff).to_sql_fragment_for(2, Dialect::Postgres);
  assert_eq!(sql, r#"datetime("memories"."created_at") >= datetime($2)"#);
  assert_eq!(params, vec![Value::Text("2026-09-18T10:00:00Z".to_owned())]);
  Ok(())
}

#[test]
fn like_without_an_escape_renders_the_same_as_the_column_operator() {
  let scalar = Scalar::col(&BODY).like(Scalar::bind("%cotton%"));
  let column = BODY.like("%cotton%");

  assert_eq!(
    scalar.to_sql_fragment_for(1, Dialect::Sqlite),
    column.to_sql_fragment_for(1, Dialect::Sqlite)
  );
  assert_eq!(
    scalar.to_sql_fragment_for(1, Dialect::Sqlite).0,
    r#""memories"."body" LIKE ?1"#
  );
}

#[test]
fn like_escape_binds_the_pattern_then_the_escape_character() {
  let (sql, params) = BODY
    .like_escape(r"%100\%%", '\\')
    .to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#""memories"."body" LIKE ?1 ESCAPE ?2"#);
  assert_eq!(
    params,
    vec![
      Value::Text(r"%100\%%".to_owned()),
      Value::Text("\\".to_owned())
    ]
  );

  let (pg_sql, _) = BODY
    .like_escape("x", '!')
    .to_sql_fragment_for(4, Dialect::Postgres);
  assert_eq!(pg_sql, r#""memories"."body" LIKE $4 ESCAPE $5"#);
}

#[test]
fn like_escape_continues_numbering_after_an_earlier_conjunct() {
  let combined = Expr::raw(
    "\"memories\".\"id\" = ?",
    vec![Value::Text("m1".to_owned())],
  )
  .and(BODY.like_escape("a", '\\'));

  let (sql, params) = combined.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"("memories"."id" = ?1 AND "memories"."body" LIKE ?2 ESCAPE ?3)"#
  );
  assert_eq!(params.len(), 3);
}

#[test]
fn a_literal_pattern_escapes_every_wildcard_and_the_escape_itself() {
  assert_eq!(like_pattern_literal("100%", '\\'), r"100\%");
  assert_eq!(like_pattern_literal("a_b", '\\'), r"a\_b");
  assert_eq!(like_pattern_literal(r"a\b", '\\'), r"a\\b");
  assert_eq!(like_pattern_literal("plain", '\\'), "plain");
  assert_eq!(like_pattern_literal("", '\\'), "");
  // A degenerate escape character is still self-consistent.
  assert_eq!(like_pattern_literal("50%", '%'), "50%%");
}
