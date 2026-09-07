//! Tests for CommonOps: eq, ne, is_null, is_not_null, in_list, not_in, and/or.
//!
//! # Public API
//!
//! Tests for Column<T> CommonOps trait methods and Expr combinators.

use toolu_orm_core::column::{Boolean, Integer, Text, Timestamp};
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;

// ── Column::new ──────────────────────────────────────────────────────────────

#[test]
fn column_new_stores_table_and_name() {
  let col: Column<Text> = Column::new("users", "name");
  assert_eq!(col.table, "users");
  assert_eq!(col.name, "name");
}

#[test]
fn column_qualified_format() {
  let col: Column<Integer> = Column::new("orders", "total");
  assert_eq!(col.qualified(), r#""orders"."total""#);
}

// ── CommonOps::eq ─────────────────────────────────────────────────────────

#[test]
fn text_column_eq_produces_correct_sql() {
  let col: Column<Text> = Column::new("users", "email");
  let expr = col.eq("alice@example.com");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."email" = ?1"#);
  assert_eq!(params, vec![Value::from("alice@example.com")]);
}

// ── CommonOps::in_list ────────────────────────────────────────────────────

#[test]
fn column_in_list_produces_correct_sql() {
  let col: Column<Integer> = Column::new("orders", "status");
  let values = vec![Value::from(1i32), Value::from(2i32), Value::from(3i32)];
  let expr = col.in_list(&values);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""orders"."status" IN (?1, ?2, ?3)"#);
  assert_eq!(
    params,
    vec![Value::from(1i32), Value::from(2i32), Value::from(3i32)]
  );
}

// ── CommonOps::is_null ────────────────────────────────────────────────────

#[test]
fn column_is_null_produces_no_params() {
  let col: Column<Text> = Column::new("users", "deleted_at");
  let expr = col.is_null();
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."deleted_at" IS NULL"#);
  assert!(params.is_empty());
}

// ── Expr::and / Expr::or ──────────────────────────────────────────────────

#[test]
fn expr_and_produces_correct_sql_with_param_numbering() {
  let col_a: Column<Text> = Column::new("users", "name");
  let col_b: Column<Integer> = Column::new("users", "age");
  let left = col_a.eq("alice");
  let right = col_b.gt(18);
  let combined = left.and(right);
  let (sql, params) = combined.to_sql_fragment(1);
  assert_eq!(sql, r#"("users"."name" = ?1 AND "users"."age" > ?2)"#);
  assert_eq!(params, vec![Value::from("alice"), Value::from(18i32)]);
}

#[test]
fn expr_or_produces_correct_sql_with_param_numbering() {
  let col_a: Column<Text> = Column::new("users", "role");
  let col_b: Column<Text> = Column::new("users", "status");
  let left = col_a.eq("admin");
  let right = col_b.eq("active");
  let combined = left.or(right);
  let (sql, params) = combined.to_sql_fragment(1);
  assert_eq!(sql, r#"("users"."role" = ?1 OR "users"."status" = ?2)"#);
  assert_eq!(params, vec![Value::from("admin"), Value::from("active")]);
}

// CommonOps::ne
#[test]
fn column_ne_produces_correct_sql() {
  let col: Column<Text> = Column::new("users", "status");
  let expr = col.ne("banned");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."status" != ?1"#);
  assert_eq!(params, vec![Value::from("banned")]);
}

// CommonOps::not_in
#[test]
fn column_not_in_produces_correct_sql() {
  let col: Column<Text> = Column::new("users", "role");
  let values = vec![Value::from("admin"), Value::from("moderator")];
  let expr = col.not_in(&values);
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."role" NOT IN (?1, ?2)"#);
  assert_eq!(params, vec![Value::from("admin"), Value::from("moderator")]);
}

// CommonOps::is_not_null
#[test]
fn column_is_not_null_produces_correct_sql() {
  let col: Column<Text> = Column::new("users", "email");
  let expr = col.is_not_null();
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."email" IS NOT NULL"#);
  assert!(params.is_empty());
}

// ── Compile-time trait enforcement ────────────────────────────────────────

// Column<Integer> has CommonOps and NumericOps
fn _assert_integer_has_numeric_ops(col: &Column<Integer>) -> toolu_orm_core::expr::Expr {
  col.gt(0)
}

fn _assert_integer_has_common_ops(col: &Column<Integer>) -> toolu_orm_core::expr::Expr {
  col.eq(0i32)
}

// Column<Boolean> has CommonOps only
fn _assert_boolean_has_common_ops(col: &Column<Boolean>) -> toolu_orm_core::expr::Expr {
  col.eq(true)
}

// Column<Timestamp> has NumericOps
fn _assert_timestamp_has_numeric_ops(col: &Column<Timestamp>) -> toolu_orm_core::expr::Expr {
  col.gt(1_000_000i64)
}

// Verify traits are implemented via static dispatch
fn _assert_text_ops_on_text<T: toolu_orm_core::query_column::TextOps>(_: &T) {}
fn _assert_numeric_ops_on_integer<T: NumericOps>(_: &T) {}
fn _assert_common_ops_on_any<T: CommonOps>(_: &T) {}
