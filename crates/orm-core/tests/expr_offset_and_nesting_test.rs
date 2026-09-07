//! Coverage for `Expr::to_sql_fragment_for` at a non-default start offset (3):
//! every typed-column operator, nested `and`/`or` precedence, and empty `in_list`.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;

fn text_col() -> Column<Text> {
  Column::new("users", "name")
}

fn int_col() -> Column<Integer> {
  Column::new("users", "age")
}

// ── Comparison operators (eq, ne, like, gt, lt, gte, lte) ────────────────────

#[test]
fn sqlite_all_comparison_operators_at_offset_three() {
  let (sql, params) = text_col()
    .eq("alice")
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."name" = ?3"#);
  assert_eq!(params, vec![Value::from("alice")]);

  let (sql, _) = text_col().ne("bob").to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."name" != ?3"#);

  let (sql, _) = text_col()
    .like("%al%")
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."name" LIKE ?3"#);

  let (sql, _) = int_col().gt(18).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" > ?3"#);

  let (sql, _) = int_col().lt(18).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" < ?3"#);

  let (sql, _) = int_col().gte(18).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" >= ?3"#);

  let (sql, _) = int_col().lte(18).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" <= ?3"#);
}

#[test]
fn postgres_all_comparison_operators_at_offset_three() {
  let (sql, params) = text_col()
    .eq("alice")
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."name" = $3"#);
  assert_eq!(params, vec![Value::from("alice")]);

  let (sql, _) = text_col()
    .ne("bob")
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."name" != $3"#);

  let (sql, _) = text_col()
    .like("%al%")
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."name" LIKE $3"#);

  let (sql, _) = int_col().gt(18).to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" > $3"#);

  let (sql, _) = int_col().lt(18).to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" < $3"#);

  let (sql, _) = int_col().gte(18).to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" >= $3"#);

  let (sql, _) = int_col().lte(18).to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" <= $3"#);
}

// ── in_list / not_in / between (two params each) ─────────────────────────────

#[test]
fn sqlite_in_list_not_in_and_between_at_offset_three() {
  let values = vec![Value::from(1i32), Value::from(2i32)];

  let (sql, params) = int_col()
    .in_list(&values)
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" IN (?3, ?4)"#);
  assert_eq!(params, values);

  let (sql, params) = int_col()
    .not_in(&values)
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" NOT IN (?3, ?4)"#);
  assert_eq!(params, values);

  let (sql, params) = int_col()
    .between(1, 10)
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."age" BETWEEN ?3 AND ?4"#);
  assert_eq!(params, vec![Value::from(1i32), Value::from(10i32)]);
}

#[test]
fn postgres_in_list_not_in_and_between_at_offset_three() {
  let values = vec![Value::from(1i32), Value::from(2i32)];

  let (sql, params) = int_col()
    .in_list(&values)
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" IN ($3, $4)"#);
  assert_eq!(params, values);

  let (sql, params) = int_col()
    .not_in(&values)
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" NOT IN ($3, $4)"#);
  assert_eq!(params, values);

  let (sql, params) = int_col()
    .between(1, 10)
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."age" BETWEEN $3 AND $4"#);
  assert_eq!(params, vec![Value::from(1i32), Value::from(10i32)]);
}

// ── is_null (no params) ───────────────────────────────────────────────────────

#[test]
fn is_null_produces_no_params_regardless_of_offset_or_dialect() {
  let (sql, params) = text_col().is_null().to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, r#""users"."name" IS NULL"#);
  assert!(params.is_empty());

  let (sql, params) = text_col()
    .is_null()
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""users"."name" IS NULL"#);
  assert!(params.is_empty());
}

// ── Nested (a AND b) OR c keeps precedence-preserving parentheses ────────────

#[test]
fn nested_and_or_preserves_parentheses_sqlite() {
  let a = text_col().eq("alice");
  let b = int_col().gt(18);
  let c = text_col().eq("bob");
  let combined = a.and(b).or(c);

  let (sql, params) = combined.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"(("users"."name" = ?3 AND "users"."age" > ?4) OR "users"."name" = ?5)"#
  );
  assert_eq!(
    params,
    vec![Value::from("alice"), Value::from(18i32), Value::from("bob")]
  );
}

#[test]
fn nested_and_or_preserves_parentheses_postgres() {
  let a = text_col().eq("alice");
  let b = int_col().gt(18);
  let c = text_col().eq("bob");
  let combined = a.and(b).or(c);

  let (sql, params) = combined.to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"(("users"."name" = $3 AND "users"."age" > $4) OR "users"."name" = $5)"#
  );
  assert_eq!(
    params,
    vec![Value::from("alice"), Value::from(18i32), Value::from("bob")]
  );
}

// ── Empty in_list / not_in render as tautology/contradiction, not `IN ()` ────

#[test]
fn empty_in_list_sqlite_renders_contradiction() {
  let (sql, params) = int_col()
    .in_list(&[])
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, "1 = 0");
  assert!(params.is_empty());
}

#[test]
fn empty_in_list_postgres_renders_contradiction() {
  let (sql, params) = int_col()
    .in_list(&[])
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, "1 = 0");
  assert!(params.is_empty());
}

#[test]
fn empty_not_in_sqlite_renders_tautology() {
  let (sql, params) = int_col()
    .not_in(&[])
    .to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, "1 = 1");
  assert!(params.is_empty());
}

#[test]
fn empty_not_in_postgres_renders_tautology() {
  let (sql, params) = int_col()
    .not_in(&[])
    .to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, "1 = 1");
  assert!(params.is_empty());
}
