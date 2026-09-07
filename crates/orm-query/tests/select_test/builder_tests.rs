use super::fixtures::{CREATED_AT, EMAIL, ID, ORG_ID, PIPELINE_USER_ID};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::JoinCondition;
use toolu_orm_core::query_column::{ColumnRef, CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

#[test]
fn select_all_columns() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id", "email" FROM "users""#);
  assert!(params.is_empty());
}

#[test]
fn select_with_filter() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .filter(ORG_ID.eq("org123"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id", "email" FROM "users" WHERE "users"."org_id" = ?1"#
  );
  assert_eq!(params, vec![Value::Text("org123".to_owned())]);
}

#[test]
fn select_with_multiple_filters_are_anded() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(ORG_ID.eq("org123"))
    .filter(EMAIL.like("%@example.com"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id" FROM "users" WHERE "users"."org_id" = ?1 AND "users"."email" LIKE ?2"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("org123".to_owned()),
      Value::Text("%@example.com".to_owned()),
    ]
  );
}

#[test]
fn select_with_order_by() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .order_by(CREATED_AT.desc())
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id" FROM "users" ORDER BY "users"."created_at" DESC"#
  );
  assert!(params.is_empty());
}

#[test]
fn select_with_limit_offset() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(10)
    .offset(20)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1 OFFSET ?2"#);
  assert_eq!(params, vec![Value::Integer(10), Value::Integer(20)]);
}

#[test]
fn select_count() {
  let (sql, params) = SelectBuilder::new("users")
    .filter(ORG_ID.eq("org123"))
    .to_count_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM "users" WHERE "users"."org_id" = ?1"#
  );
  assert_eq!(params, vec![Value::Text("org123".to_owned())]);
}

#[test]
fn select_with_inner_join() {
  let on = ID.equals(&PIPELINE_USER_ID);
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .join("pipelines", on)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id", "email" FROM "users" INNER JOIN "pipelines" ON "users"."id" = "pipelines"."user_id""#
  );
  assert!(params.is_empty());
}

#[test]
fn select_with_left_join() {
  let on = ID.equals(&PIPELINE_USER_ID);
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .left_join("pipelines", on)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id", "email" FROM "users" LEFT JOIN "pipelines" ON "users"."id" = "pipelines"."user_id""#
  );
  assert!(params.is_empty());
}

#[test]
fn select_raw_with_column_expr() {
  let (sql, params) = SelectBuilder::raw()
    .column_expr(
      "(SELECT COUNT(*) FROM pipelines WHERE pipelines.user_id = users.id)",
      "pipeline_count",
    )
    .column_expr(
      "(SELECT MAX(created_at) FROM pipelines WHERE pipelines.user_id = users.id)",
      "last_pipeline_at",
    )
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT (SELECT COUNT(*) FROM pipelines WHERE pipelines.user_id = users.id) AS "pipeline_count", (SELECT MAX(created_at) FROM pipelines WHERE pipelines.user_id = users.id) AS "last_pipeline_at""#
  );
  assert!(params.is_empty());
}

#[test]
fn select_with_typed_columns() {
  let cols: &[&dyn ColumnRef] = &[&ID, &EMAIL];
  let (sql, params) = SelectBuilder::new("users")
    .columns_typed(cols)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id", "email" FROM "users""#);
  assert!(params.is_empty());
}

#[test]
fn select_exists() {
  let (sql, params) = SelectBuilder::new("users")
    .filter(ORG_ID.eq("org123"))
    .to_exists_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT EXISTS(SELECT 1 FROM "users" WHERE "users"."org_id" = ?1)"#
  );
  assert_eq!(params, vec![Value::Text("org123".to_owned())]);
}

#[test]
fn select_with_full_query() {
  let on: JoinCondition = ID.equals(&PIPELINE_USER_ID);
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id", "email"])
    .filter(ORG_ID.eq("org123"))
    .filter(CREATED_AT.gt(0i32))
    .join("pipelines", on)
    .order_by(CREATED_AT.desc())
    .limit(10)
    .offset(0)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"SELECT "id", "email" FROM "users" INNER JOIN "pipelines" ON "users"."id" = "pipelines"."user_id" WHERE "users"."org_id" = ?1 AND "users"."created_at" > ?2 ORDER BY "users"."created_at" DESC LIMIT ?3 OFFSET ?4"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("org123".to_owned()),
      Value::Integer(0),
      Value::Integer(10),
      Value::Integer(0),
    ]
  );
}
