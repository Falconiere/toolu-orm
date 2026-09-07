use super::fixtures::ID;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

#[test]
fn select_to_sql_for_postgres_uses_dollar_params() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "name"])
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"SELECT "id", "name" FROM "users" WHERE "users"."id" = $1"#
  );
  assert_eq!(params, vec![Value::Text("user-1".to_owned())]);
}

#[test]
fn select_to_sql_for_sqlite_uses_question_params() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "name"])
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id", "name" FROM "users" WHERE "users"."id" = ?1"#
  );
  assert_eq!(params, vec![Value::Text("user-1".to_owned())]);
}

#[test]
fn select_limit_offset_postgres_params() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(ID.eq("u1"))
    .limit(10)
    .offset(20)
    .to_sql_for(Dialect::Postgres);
  assert!(sql.contains("$1"));
  assert!(sql.contains("$2"));
  assert!(sql.contains("$3"));
  assert_eq!(params.len(), 3);
}

#[test]
fn select_to_sql_delegates_to_current() {
  let builder = SelectBuilder::new("users").columns_raw(&["id"]);
  let (via_default, _) = builder.to_sql();
  let (via_explicit, _) = builder.to_sql_for(Dialect::CURRENT);
  assert_eq!(via_default, via_explicit);
}
