use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::update::UpdateBuilder;

const ID: Column<Text> = Column::new("users", "id");
const EMAIL: Column<Text> = Column::new("users", "email");
const NAME: Column<Text> = Column::new("users", "name");
const UPDATED_AT: Column<Integer> = Column::new("users", "updated_at");
const AGE: Column<Integer> = Column::new("users", "age");

// ── 1. basic_update_with_filter ───────────────────────────────────────────────

#[test]
fn basic_update_with_filter() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "new@example.com")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1 WHERE "users"."id" = ?2"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("new@example.com".to_owned()),
      Value::Text("user-1".to_owned()),
    ]
  );
}

// ── 2. update_with_set_expr ───────────────────────────────────────────────────

#[test]
fn update_with_set_expr() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "new@example.com")
    .set_expr(&UPDATED_AT, "unixepoch()")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1, "updated_at" = unixepoch() WHERE "users"."id" = ?2"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("new@example.com".to_owned()),
      Value::Text("user-1".to_owned()),
    ]
  );
}

// ── 3. update_with_multiple_sets ──────────────────────────────────────────────

#[test]
fn update_with_multiple_sets() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "new@example.com")
    .set(&NAME, "Alice")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1, "name" = ?2 WHERE "users"."id" = ?3"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("new@example.com".to_owned()),
      Value::Text("Alice".to_owned()),
      Value::Text("user-1".to_owned()),
    ]
  );
}

// ── 4. update_with_multiple_filters ───────────────────────────────────────────

#[test]
fn update_with_multiple_filters() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "new@example.com")
    .filter(ID.eq("user-1"))
    .filter(AGE.gt(18i32))
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1 WHERE "users"."id" = ?2 AND "users"."age" > ?3"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("new@example.com".to_owned()),
      Value::Text("user-1".to_owned()),
      Value::Integer(18),
    ]
  );
}

// ── 5. mixed_set_and_set_expr_params_order ────────────────────────────────────

#[test]
fn mixed_set_and_set_expr_params_order() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "x@y.com")
    .set_expr(&UPDATED_AT, "unixepoch()")
    .set(&NAME, "Bob")
    .filter(ID.eq("user-2"))
    .to_sql_for(Dialect::Sqlite);
  // SET value params come first (?1, ?2 for email and name), then WHERE params (?3 for id)
  // set_expr produces no param, so params are: email, name, then filter id
  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1, "updated_at" = unixepoch(), "name" = ?2 WHERE "users"."id" = ?3"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("x@y.com".to_owned()),
      Value::Text("Bob".to_owned()),
      Value::Text("user-2".to_owned()),
    ]
  );
}

#[test]
fn update_to_sql_for_postgres_uses_dollar_params() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "new@b.com")
    .filter(ID.eq("user-1"))
    .to_sql_for(Dialect::Postgres);
  assert!(sql.contains("$1"));
  assert!(sql.contains("$2"));
  assert_eq!(params.len(), 2);
}

#[test]
fn update_to_sql_delegates_to_current() {
  let builder = UpdateBuilder::new("users").set(&EMAIL, "x@y.com");
  let (via_default, _) = builder.to_sql();
  let (via_explicit, _) = builder.to_sql_for(Dialect::CURRENT);
  assert_eq!(via_default, via_explicit);
}
