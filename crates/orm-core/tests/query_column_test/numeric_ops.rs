//! Tests for NumericOps: gt, lt, lte, gte, between, BigInt, SmallInt, Real.
//!
//! # Public API
//!
//! Tests for Column<T> NumericOps trait methods and param offset handling.

use toolu_orm_core::column::{BigInt, Integer, Real, SmallInt};
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;

// ── NumericOps::gt ────────────────────────────────────────────────────────

#[test]
fn integer_column_gt_produces_correct_sql() {
  let col: Column<Integer> = Column::new("users", "age");
  let expr = col.gt(18);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."age" > ?1"#);
  assert_eq!(params, vec![Value::from(18i32)]);
}

// ── NumericOps::between ───────────────────────────────────────────────────

#[test]
fn column_between_produces_correct_sql() {
  let col: Column<Integer> = Column::new("products", "price");
  let expr = col.between(10, 100);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""products"."price" BETWEEN ?1 AND ?2"#);
  assert_eq!(params, vec![Value::from(10i32), Value::from(100i32)]);
}

// NumericOps::lt, lte, gte
#[test]
fn integer_column_lt_produces_correct_sql() {
  let col: Column<Integer> = Column::new("products", "stock");
  let expr = col.lt(10);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""products"."stock" < ?1"#);
  assert_eq!(params, vec![Value::from(10i32)]);
}

#[test]
fn integer_column_lte_produces_correct_sql() {
  let col: Column<Integer> = Column::new("products", "stock");
  let expr = col.lte(10);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""products"."stock" <= ?1"#);
  assert_eq!(params, vec![Value::from(10i32)]);
}

#[test]
fn integer_column_gte_produces_correct_sql() {
  let col: Column<Integer> = Column::new("users", "age");
  let expr = col.gte(18);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."age" >= ?1"#);
  assert_eq!(params, vec![Value::from(18i32)]);
}

// Real column has NumericOps
#[test]
fn real_column_gt_produces_correct_sql() {
  let col: Column<Real> = Column::new("products", "price");
  let expr = col.gt(9.99f64);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""products"."price" > ?1"#);
  assert_eq!(params, vec![Value::from(9.99f64)]);
}

// BigInt and SmallInt have NumericOps
#[test]
fn bigint_column_has_numeric_ops() {
  let col: Column<BigInt> = Column::new("stats", "count");
  let expr = col.gt(1000i64);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""stats"."count" > ?1"#);
  assert_eq!(params, vec![Value::from(1000i64)]);
}

#[test]
fn smallint_column_has_numeric_ops() {
  let col: Column<SmallInt> = Column::new("settings", "priority");
  let expr = col.lt(10i32);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""settings"."priority" < ?1"#);
  assert_eq!(params, vec![Value::from(10i32)]);
}

// Param numbering starts at offset
#[test]
fn to_sql_fragment_respects_start_offset() {
  let col: Column<Integer> = Column::new("items", "qty");
  let expr = col.gt(5);
  let (sql, params) = expr.to_sql_fragment(3);
  assert_eq!(sql, r#""items"."qty" > ?3"#);
  assert_eq!(params, vec![Value::from(5i32)]);
}

// Nested and with offset
#[test]
fn nested_and_respects_start_offset() {
  use toolu_orm_core::column::Text;
  let col_a: Column<Text> = Column::new("t", "a");
  let col_b: Column<Integer> = Column::new("t", "b");
  let combined = col_a.eq("x").and(col_b.gt(0));
  let (sql, params) = combined.to_sql_fragment(5);
  assert_eq!(sql, r#"("t"."a" = ?5 AND "t"."b" > ?6)"#);
  assert_eq!(params, vec![Value::from("x"), Value::from(0i32)]);
}
