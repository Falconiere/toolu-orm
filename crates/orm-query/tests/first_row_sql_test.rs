//! `to_first_row_sql_for`: the bounded query `fetch_one` / `fetch_optional`
//! send, rendered without a database.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

const ID: Column<Text> = Column::new("users", "id");
const ORG_ID: Column<Text> = Column::new("users", "org_id");
const CREATED_AT: Column<Integer> = Column::new("users", "created_at");

#[test]
fn no_limit_becomes_limit_one() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1"#);
  assert_eq!(params, vec![Value::Integer(1)]);
}

#[test]
fn positive_limit_is_clamped_to_one() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(5)
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1"#);
  assert_eq!(params, vec![Value::Integer(1)]);
}

#[test]
fn explicit_limit_zero_is_preserved() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(0)
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1"#);
  assert_eq!(params, vec![Value::Integer(0)]);
}

/// A negative limit keeps its driver-defined meaning — SQLite reads it as "no
/// limit", Postgres rejects it — so the first-row form passes it through
/// rather than changing an observable result on either driver.
#[test]
fn negative_limit_is_passed_through_unchanged() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(-1)
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1"#);
  assert_eq!(params, vec![Value::Integer(-1)]);
}

#[test]
fn filters_order_and_offset_are_preserved_with_continued_parameters() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(ORG_ID.eq("org123"))
    .filter(CREATED_AT.gt(100))
    .order_by(CREATED_AT.desc())
    .offset(7)
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "id" FROM "users" WHERE "users"."org_id" = ?1 AND "#,
      r#""users"."created_at" > ?2 ORDER BY "users"."created_at" DESC "#,
      r#"LIMIT ?3 OFFSET ?4"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("org123".to_owned()),
      Value::Integer(100),
      Value::Integer(1),
      Value::Integer(7),
    ]
  );
}

#[test]
fn postgres_renders_dollar_placeholders() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .filter(ID.eq("u1"))
    .limit(9)
    .offset(3)
    .to_first_row_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"SELECT "id" FROM "users" WHERE "users"."id" = $1 LIMIT $2 OFFSET $3"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("u1".to_owned()),
      Value::Integer(1),
      Value::Integer(3),
    ]
  );
}

/// The unbounded renderer is untouched: same query, the caller's own limit.
#[test]
fn to_sql_for_still_renders_the_builders_own_limit() {
  let (sql, params) = SelectBuilder::new("users")
    .columns_raw(&["id"])
    .limit(5)
    .offset(2)
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ?1 OFFSET ?2"#);
  assert_eq!(params, vec![Value::Integer(5), Value::Integer(2)]);
}

/// `SelectBuilder::raw` has no `FROM`; the bound still renders legally.
#[test]
fn raw_builder_still_renders_a_bound() {
  let (sql, params) = SelectBuilder::raw()
    .column_expr("1", "one")
    .to_first_row_sql_for(Dialect::Sqlite);
  assert_eq!(sql, r#"SELECT 1 AS "one" LIMIT ?1"#);
  assert_eq!(params, vec![Value::Integer(1)]);
}
