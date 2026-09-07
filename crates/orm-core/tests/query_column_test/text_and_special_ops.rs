//! Tests for TextOps (like), JsonExpr, raw expressions, and special column types.
//!
//! # Public API
//!
//! Tests for like, json_extract, Expr::raw, Varchar, Uuid, Date, Time columns.

use toolu_orm_core::column::{Date, Text, Time, Uuid, Varchar};
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{Column, NumericOps, TextOps};
use toolu_orm_core::value::Value;

// ── TextOps::like ─────────────────────────────────────────────────────────

#[test]
fn text_column_like_produces_correct_sql() {
  let col: Column<Text> = Column::new("posts", "title");
  let expr = col.like("%pattern%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""posts"."title" LIKE ?1"#);
  assert_eq!(params, vec![Value::from("%pattern%")]);
}

// ── OrderBy ───────────────────────────────────────────────────────────────

#[test]
fn column_desc_produces_correct_sql() {
  use toolu_orm_core::expr::OrderBy;
  let col: Column<Text> = Column::new("posts", "created_at");
  let order: OrderBy = col.desc();
  assert_eq!(order.to_sql(), r#""posts"."created_at" DESC"#);
}

#[test]
fn column_asc_produces_correct_sql() {
  use toolu_orm_core::column::Integer;
  use toolu_orm_core::expr::OrderBy;
  let col: Column<Integer> = Column::new("users", "age");
  let order: OrderBy = col.asc();
  assert_eq!(order.to_sql(), r#""users"."age" ASC"#);
}

// ── JoinCondition ─────────────────────────────────────────────────────────

#[test]
fn column_equals_produces_join_condition_sql() {
  use toolu_orm_core::expr::JoinCondition;
  let col_a: Column<Uuid> = Column::new("orders", "user_id");
  let col_b: Column<Uuid> = Column::new("users", "id");
  let join: JoinCondition = col_a.equals(&col_b);
  assert_eq!(join.to_sql(), r#""orders"."user_id" = "users"."id""#);
}

// ── Expr::raw ─────────────────────────────────────────────────────────────

#[test]
fn expr_raw_replaces_bare_question_marks_with_numbered_params() {
  let params = vec![Value::from("active"), Value::from(42i32)];
  let expr = Expr::raw("status = ? AND count > ?", params);
  let (sql, out_params) = expr.to_sql_fragment(1);
  assert_eq!(sql, "status = ?1 AND count > ?2");
  assert_eq!(out_params, vec![Value::from("active"), Value::from(42i32)]);
}

// ── JsonExpr ─────────────────────────────────────────────────────────────

#[test]
fn expr_json_extract_eq_produces_correct_sql() {
  use toolu_orm_core::query_column::CommonOps;
  let col: Column<Text> = Column::new("records", "data");
  let json_expr = Expr::json_extract(&col, "$.key");
  let expr = json_expr.eq("value");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#"json_extract("records"."data", '$.key') = ?1"#);
  assert_eq!(params, vec![Value::from("value")]);
}

// Varchar also gets TextOps
#[test]
fn varchar_column_like_produces_correct_sql() {
  let col: Column<Varchar<255>> = Column::new("users", "username");
  let expr = col.like("alice%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."username" LIKE ?1"#);
  assert_eq!(params, vec![Value::from("alice%")]);
}

// Uuid column has TextOps
#[test]
fn uuid_column_like_produces_correct_sql() {
  let col: Column<Uuid> = Column::new("users", "id");
  let expr = col.like("abc%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""users"."id" LIKE ?1"#);
  assert_eq!(params, vec![Value::from("abc%")]);
}

// Date column has both TextOps and NumericOps
#[test]
fn date_column_like_produces_correct_sql() {
  let col: Column<Date> = Column::new("events", "event_date");
  let expr = col.like("2024-%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""events"."event_date" LIKE ?1"#);
  assert_eq!(params, vec![Value::from("2024-%")]);
}

#[test]
fn date_column_gt_produces_correct_sql() {
  let col: Column<Date> = Column::new("events", "event_date");
  let expr = col.gt("2024-01-01");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""events"."event_date" > ?1"#);
  assert_eq!(params, vec![Value::from("2024-01-01")]);
}

// Time column has both TextOps and NumericOps
#[test]
fn time_column_has_text_and_numeric_ops() {
  let col: Column<Time> = Column::new("slots", "start_time");
  let expr = col.like("08:%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#""slots"."start_time" LIKE ?1"#);
  assert_eq!(params, vec![Value::from("08:%")]);
}

// JsonExpr::like
#[test]
fn expr_json_extract_like_produces_correct_sql() {
  let col: Column<Text> = Column::new("records", "data");
  let json_expr = Expr::json_extract(&col, "$.name");
  let expr = json_expr.like("%alice%");
  let (sql, params) = expr.to_sql_fragment(1);
  assert_eq!(sql, r#"json_extract("records"."data", '$.name') LIKE ?1"#);
  assert_eq!(params, vec![Value::from("%alice%")]);
}

// Raw expr does not re-number already-numbered params
#[test]
fn expr_raw_does_not_renumber_already_numbered_params() {
  let params = vec![Value::from("active")];
  let expr = Expr::raw("status = ?1", params);
  let (sql, out_params) = expr.to_sql_fragment(1);
  assert_eq!(sql, "status = ?1");
  assert_eq!(out_params, vec![Value::from("active")]);
}
