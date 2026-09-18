//! Arithmetic and string concatenation between scalars.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

const HITS: Column<Integer> = Column::new("memories", "access_count");
const BODY: Column<Text> = Column::new("memories", "body");

#[test]
fn a_column_plus_one_renders_parenthesized_with_one_bind() {
  let (sql, params) =
    (Scalar::col(&HITS) + Scalar::bind(1)).to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#"("memories"."access_count" + ?1)"#);
  assert_eq!(params, vec![Value::Integer(1)]);
}

#[test]
fn every_operator_renders_its_symbol() {
  let cases = [
    (Scalar::bind(6) + Scalar::bind(2), "+"),
    (Scalar::bind(6) - Scalar::bind(2), "-"),
    (Scalar::bind(6) * Scalar::bind(2), "*"),
    (Scalar::bind(6) / Scalar::bind(2), "/"),
    (Scalar::bind(6).concat(Scalar::bind(2)), "||"),
  ];
  for (scalar, op) in cases {
    assert_eq!(
      scalar.to_sql_fragment_for(1, Dialect::Sqlite).0,
      format!("(?1 {op} ?2)")
    );
  }
}

#[test]
fn nesting_keeps_parentheses_and_orders_binds_left_to_right() {
  let expr = Scalar::col(&BODY)
    .concat(Scalar::bind(":"))
    .concat(Scalar::bind("tail"));

  let (sql, params) = expr.to_sql_fragment_for(2, Dialect::Postgres);
  assert_eq!(sql, r#"(("memories"."body" || $2) || $3)"#);
  assert_eq!(
    params,
    vec![Value::Text(":".to_owned()), Value::Text("tail".to_owned())]
  );
}

#[test]
fn a_raw_operand_continues_numbering_after_its_left_sibling() {
  let expr = Scalar::bind(10) + Scalar::raw("(? + ?)", vec![Value::Integer(1), Value::Integer(2)]);

  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "(?1 + (?2 + ?3))");
  assert_eq!(params.len(), 3);
}
