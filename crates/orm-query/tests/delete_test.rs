use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;

const ID: Column<Text> = Column::new("users", "id");
const AGE: Column<Integer> = Column::new("users", "age");
const ORG_ID: Column<Text> = Column::new("users", "org_id");

// ── 1. basic_delete_with_filter ───────────────────────────────────────────────

#[test]
fn basic_delete_with_filter() {
  let (sql, params) = DeleteBuilder::new("users")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"DELETE FROM "users" WHERE "users"."id" = ?1"#);
  assert_eq!(params, vec![Value::Text("user-1".to_owned())]);
}

// ── 2. delete_with_multiple_filters ───────────────────────────────────────────

#[test]
fn delete_with_multiple_filters() {
  let (sql, params) = DeleteBuilder::new("users")
    .filter(ORG_ID.eq("org-1"))
    .filter(AGE.lt(18i32))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"DELETE FROM "users" WHERE "users"."org_id" = ?1 AND "users"."age" < ?2"#
  );
  assert_eq!(
    params,
    vec![Value::Text("org-1".to_owned()), Value::Integer(18),]
  );
}

// ── 3. delete_no_filter ───────────────────────────────────────────────────────

#[test]
fn delete_no_filter() {
  let (sql, params) = DeleteBuilder::new("users").to_sql();
  assert_eq!(sql, r#"DELETE FROM "users""#);
  let empty: Vec<Value> = Vec::new();
  assert_eq!(params, empty);
}

#[test]
fn delete_to_sql_for_postgres_uses_dollar_params() {
  let (sql, params) = DeleteBuilder::new("users")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Postgres);
  assert_eq!(sql, r#"DELETE FROM "users" WHERE "users"."id" = $1"#);
  assert_eq!(params, vec![Value::Text("user-1".to_owned())]);
}

#[test]
fn delete_to_sql_delegates_to_current() {
  let builder = DeleteBuilder::new("users").filter(ID.eq("u1"));
  let (via_default, _) = builder.to_sql();
  let (via_explicit, _) = builder.to_sql_for(Dialect::CURRENT);
  assert_eq!(via_default, via_explicit);
}
