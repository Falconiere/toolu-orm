use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Expr;
use toolu_orm_core::value::Value;

// ── Comparison ───────────────────────────────────────────────────────────────

#[test]
fn sqlite_comparison_uses_question_mark() {
  let expr = Expr::raw("name = ?", vec![Value::Text("alice".to_owned())]);
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "name = ?1");
  assert_eq!(params, vec![Value::Text("alice".to_owned())]);
}

#[test]
fn postgres_comparison_uses_dollar_sign() {
  let expr = Expr::raw("name = ?", vec![Value::Text("alice".to_owned())]);
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(sql, "name = $1");
  assert_eq!(params, vec![Value::Text("alice".to_owned())]);
}

// ── Multiple params ──────────────────────────────────────────────────────────

#[test]
fn sqlite_multiple_params_sequential() {
  let expr = Expr::raw(
    "a = ? AND b = ?",
    vec![Value::Integer(1), Value::Text("x".to_owned())],
  );
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "a = ?1 AND b = ?2");
  assert_eq!(params.len(), 2);
}

#[test]
fn postgres_multiple_params_sequential() {
  let expr = Expr::raw(
    "a = ? AND b = ?",
    vec![Value::Integer(1), Value::Text("x".to_owned())],
  );
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(sql, "a = $1 AND b = $2");
  assert_eq!(params.len(), 2);
}

// ── Offset start ─────────────────────────────────────────────────────────────

#[test]
fn sqlite_respects_start_offset() {
  let expr = Expr::raw("x = ?", vec![Value::Integer(42)]);
  let (sql, _) = expr.to_sql_fragment_for(5, Dialect::Sqlite);
  assert_eq!(sql, "x = ?5");
}

#[test]
fn postgres_respects_start_offset() {
  let expr = Expr::raw("x = ?", vec![Value::Integer(42)]);
  let (sql, _) = expr.to_sql_fragment_for(5, Dialect::Postgres);
  assert_eq!(sql, "x = $5");
}

// ── to_sql_fragment delegates to CURRENT ─────────────────────────────────────

#[test]
fn to_sql_fragment_delegates_to_current() {
  let expr = Expr::raw("x = ?", vec![Value::Integer(1)]);
  let (via_default, _) = expr.to_sql_fragment(1);
  let (via_explicit, _) = expr.to_sql_fragment_for(1, Dialect::CURRENT);
  assert_eq!(via_default, via_explicit);
}

// ── No params ────────────────────────────────────────────────────────────────

#[test]
fn postgres_no_params_no_substitution() {
  let expr = Expr::raw("1 = 1", vec![]);
  let (sql, params) = expr.to_sql_fragment_for(1, Dialect::Postgres);
  assert_eq!(sql, "1 = 1");
  assert!(params.is_empty());
}
