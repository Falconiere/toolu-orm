//! Regression coverage for issue #113: `Expr::raw` nested inside `and`/`or`
//! must number its bare `?` placeholders after whatever params earlier
//! siblings already emitted, not from the raw `start` offset alone.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;

fn repo_col() -> Column<Text> {
  Column::new("memories", "repo")
}

fn age_col() -> Column<Integer> {
  Column::new("users", "age")
}

// ── AC-1 / AC-2: issue #113 repro — Comparison.and(Raw) ──────────────────────

#[test]
fn comparison_and_raw_sqlite_numbers_sequentially() {
  let predicate = repo_col().eq("comemory").and(Expr::raw(
    "datetime(created_at) >= datetime(?)",
    vec![Value::from("2026-01-01")],
  ));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"("memories"."repo" = ?1 AND datetime(created_at) >= datetime(?2))"#
  );
  assert_eq!(
    params,
    vec![Value::from("comemory"), Value::from("2026-01-01")]
  );
}

#[test]
fn comparison_and_raw_postgres_numbers_sequentially() {
  let predicate = repo_col().eq("comemory").and(Expr::raw(
    "datetime(created_at) >= datetime(?)",
    vec![Value::from("2026-01-01")],
  ));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"("memories"."repo" = $1 AND datetime(created_at) >= datetime($2))"#
  );
  assert_eq!(
    params,
    vec![Value::from("comemory"), Value::from("2026-01-01")]
  );
}

// ── AC-3: Raw first, Comparison second ───────────────────────────────────────

#[test]
fn raw_and_comparison_sqlite_numbers_sequentially() {
  let predicate = Expr::raw(
    "datetime(created_at) >= datetime(?)",
    vec![Value::from("2026-01-01")],
  )
  .and(repo_col().eq("comemory"));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"(datetime(created_at) >= datetime(?1) AND "memories"."repo" = ?2)"#
  );
  assert_eq!(
    params,
    vec![Value::from("2026-01-01"), Value::from("comemory")]
  );
}

// ── AC-4 / AC-5: two raw fragments combined with `or` ────────────────────────

#[test]
fn raw_or_raw_sqlite_numbers_sequentially() {
  let predicate =
    Expr::raw("a = ?", vec![Value::from(1i32)]).or(Expr::raw("b = ?", vec![Value::from(2i32)]));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "(a = ?1 OR b = ?2)");
  assert_eq!(params, vec![Value::from(1i32), Value::from(2i32)]);
}

#[test]
fn raw_or_raw_postgres_numbers_sequentially() {
  let predicate =
    Expr::raw("a = ?", vec![Value::from(1i32)]).or(Expr::raw("b = ?", vec![Value::from(2i32)]));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(sql, "(a = $1 OR b = $2)");
  assert_eq!(params, vec![Value::from(1i32), Value::from(2i32)]);
}

// ── AC-5: three raw siblings chained ─────────────────────────────────────────

#[test]
fn three_raw_siblings_chained_number_sequentially() {
  let predicate = Expr::raw("a = ?", vec![Value::from(1i32)])
    .and(Expr::raw("b = ?", vec![Value::from(2i32)]))
    .and(Expr::raw("c = ?", vec![Value::from(3i32)]));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "((a = ?1 AND b = ?2) AND c = ?3)");
  assert_eq!(
    params,
    vec![Value::from(1i32), Value::from(2i32), Value::from(3i32)]
  );
}

// ── AC-6: Raw nested two levels deep inside And/Or ───────────────────────────

#[test]
fn raw_nested_two_levels_in_and_or() {
  let left = repo_col()
    .eq("comemory")
    .and(Expr::raw("a = ?", vec![Value::from(1i32)]));
  let right = age_col()
    .gt(18)
    .and(Expr::raw("b = ?", vec![Value::from(2i32)]));
  let predicate = left.or(right);

  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"(("memories"."repo" = ?1 AND a = ?2) OR ("users"."age" > ?3 AND b = ?4))"#
  );
  assert_eq!(
    params,
    vec![
      Value::from("comemory"),
      Value::from(1i32),
      Value::from(18i32),
      Value::from(2i32),
    ]
  );
}

// ── AC-7: non-1 start offsets compose with prior params ──────────────────────

#[test]
fn non_default_start_offsets_compose_with_prior_params() {
  let predicate_sqlite = repo_col().eq("comemory").and(Expr::raw(
    "datetime(created_at) >= datetime(?)",
    vec![Value::from("2026-01-01")],
  ));
  let (sql, _) = predicate_sqlite.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"("memories"."repo" = ?3 AND datetime(created_at) >= datetime(?4))"#
  );

  let predicate_pg = repo_col().eq("comemory").and(Expr::raw(
    "datetime(created_at) >= datetime(?)",
    vec![Value::from("2026-01-01")],
  ));
  let (sql, _) = predicate_pg.to_sql_fragment_for(7, Dialect::Postgres);
  assert_eq!(
    sql,
    r#"("memories"."repo" = $7 AND datetime(created_at) >= datetime($8))"#
  );
}

// ── AC-8: a zero-param raw sibling does not reserve an index ────────────────

#[test]
fn zero_param_raw_sibling_does_not_reserve_an_index() {
  let predicate = repo_col()
    .eq("comemory")
    .and(Expr::raw("1 = 1", vec![]))
    .and(age_col().gt(18));
  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"(("memories"."repo" = ?1 AND 1 = 1) AND "users"."age" > ?2)"#
  );
  assert_eq!(params, vec![Value::from("comemory"), Value::from(18i32)]);
}
