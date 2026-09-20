//! SQL for `UPDATE`/`DELETE` `RETURNING` and the two `ON CONFLICT` `WHERE`
//! clauses. Driver-free, so this binary runs in the default lane. Every
//! assertion names its dialect with `to_sql_for`.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::update::UpdateBuilder;

const ID: Column<Text> = Column::new("users", "id");
const EMAIL: Column<Text> = Column::new("users", "email");
const SKU: Column<Text> = Column::new("products", "sku");
const DELETED: Column<Integer> = Column::new("products", "deleted");
const NAME: Column<Text> = Column::new("products", "name");

#[test]
fn returning_projects_update_columns_after_the_where_clause() {
  let (sql, params) = UpdateBuilder::new("users")
    .set(&EMAIL, "a@b.c")
    .filter(ID.eq("u1"))
    .returning(&ID)
    .returning(&EMAIL)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"UPDATE "users" SET "email" = ?1 WHERE "users"."id" = ?2 RETURNING "id", "email""#
  );
  assert_eq!(params.len(), 2, "RETURNING binds nothing");
}

#[test]
fn returning_projects_delete_columns_after_the_where_clause() {
  let (sql, params) = DeleteBuilder::new("users")
    .filter(ID.eq("u1"))
    .returning(&ID)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"DELETE FROM "users" WHERE "users"."id" = ?1 RETURNING "id""#
  );
  assert_eq!(params, vec![Value::Text("u1".to_owned())]);
}

#[test]
fn no_returning_call_leaves_update_and_delete_unchanged() {
  let (update, _) = UpdateBuilder::new("users")
    .set(&EMAIL, "a@b.c")
    .filter(ID.eq("u1"))
    .to_sql_for(Dialect::Sqlite);
  let (delete, _) = DeleteBuilder::new("users")
    .filter(ID.eq("u1"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    update,
    r#"UPDATE "users" SET "email" = ?1 WHERE "users"."id" = ?2"#
  );
  assert_eq!(delete, r#"DELETE FROM "users" WHERE "users"."id" = ?1"#);
}

#[test]
fn an_index_predicate_binds_before_the_assignments_and_the_guard_after() {
  let (sql, params) = InsertBuilder::new("products")
    .set(&SKU, "pen")
    .set(&DELETED, 0_i64)
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(DELETED.eq(0_i64))
        .set(&NAME, "renamed")
        .where_update(NAME.eq("open")),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "products" ("sku", "deleted") VALUES (?1, ?2) "#,
      r#"ON CONFLICT ("sku") WHERE "products"."deleted" = ?3 "#,
      r#"DO UPDATE SET "name" = ?4 WHERE "products"."name" = ?5"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("pen".to_owned()),
      Value::Integer(0),
      Value::Integer(0),
      Value::Text("renamed".to_owned()),
      Value::Text("open".to_owned()),
    ]
  );
}

#[test]
fn the_same_clauses_number_dollar_placeholders_on_postgres() {
  let (sql, _) = InsertBuilder::new("products")
    .set(&SKU, "pen")
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(DELETED.eq(0_i64))
        .set(&NAME, "renamed")
        .where_update(NAME.eq("open")),
    )
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "products" ("sku") VALUES ($1) "#,
      r#"ON CONFLICT ("sku") WHERE "products"."deleted" = $2 "#,
      r#"DO UPDATE SET "name" = $3 WHERE "products"."name" = $4"#
    )
  );
}

#[test]
fn do_nothing_discards_the_update_guard_and_keeps_the_index_predicate() {
  let (sql, params) = InsertBuilder::new("products")
    .set(&SKU, "pen")
    .on_conflict(
      OnConflict::column(&SKU)
        .where_target(Scalar::col(&DELETED).eq(Scalar::sql("0")))
        .set(&NAME, "ignored")
        .where_update(NAME.eq("open"))
        .do_nothing(),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "products" ("sku") VALUES (?1) "#,
      r#"ON CONFLICT ("sku") WHERE "products"."deleted" = 0 DO NOTHING"#
    )
  );
  assert_eq!(
    params,
    vec![Value::Text("pen".to_owned())],
    "a discarded guard and assignment bind nothing"
  );
}

#[test]
fn an_update_guard_without_an_assignment_binds_nothing() {
  let (sql, params) = InsertBuilder::new("products")
    .set(&SKU, "pen")
    .on_conflict(OnConflict::column(&SKU).where_update(NAME.eq("open")))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "products" ("sku") VALUES (?1) ON CONFLICT ("sku") DO NOTHING"#
  );
  assert_eq!(params, vec![Value::Text("pen".to_owned())]);
}
