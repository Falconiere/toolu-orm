//! Sharing across clause boundaries within one statement, and in the
//! mutation builders.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Scalar, SharedBind, SharedBindList};
use toolu_orm_core::query_column::{CommonOps, SharedOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;

use crate::seed::{DST_ID, REL, SRC_ID, WEIGHT};

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

#[test]
fn one_handle_spans_projection_two_filters_having_and_order_by() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SelectBuilder::new("edges")
    .column_scalar(Scalar::shared(&node), "asked_about")
    .column_scalar(Scalar::sum(Scalar::col(&WEIGHT)), "total")
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.ne_shared(&node))
    .group_by(&SRC_ID)
    .having(Scalar::count_star().gt(Scalar::bind(0)))
    .order_by(Scalar::shared(&node).asc())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT ?1 AS "asked_about", SUM("edges"."weight") AS "total" FROM "edges""#,
      r#" WHERE "edges"."src_id" = ?1 AND "edges"."dst_id" != ?1"#,
      r#" GROUP BY "edges"."src_id" HAVING COUNT(*) > ?2 ORDER BY ?1 ASC"#
    )
  );
  assert_eq!(params, vec![text("candidate"), Value::Integer(0)]);
}

#[test]
fn the_same_statement_shares_the_handle_with_dollar_placeholders() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SelectBuilder::new("edges")
    .column_scalar(Scalar::shared(&node), "asked_about")
    .filter(SRC_ID.eq_shared(&node))
    .filter(DST_ID.ne_shared(&node))
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT $1 AS "asked_about" FROM "edges""#,
      r#" WHERE "edges"."src_id" = $1 AND "edges"."dst_id" != $1"#
    )
  );
  assert_eq!(params, vec![text("candidate")]);
}

#[test]
fn a_join_predicate_shares_with_the_where_clause() {
  let node = SharedBind::new("candidate");

  let (sql, params) = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .join("edges", SRC_ID.equals(&DST_ID).and(SRC_ID.eq_shared(&node)))
    .filter(DST_ID.eq_shared(&node))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"SELECT "edges"."src_id" FROM "edges" INNER JOIN "edges" ON"#,
      r#" ("edges"."src_id" = "edges"."dst_id" AND "edges"."src_id" = ?1)"#,
      r#" WHERE "edges"."dst_id" = ?1"#
    )
  );
  assert_eq!(params, vec![text("candidate")]);
}

#[test]
fn an_update_shares_a_handle_between_its_set_clause_and_its_where() {
  let node = SharedBind::new("candidate");

  let (sql, params) = UpdateBuilder::new("edges")
    .set_scalar(&SRC_ID, Scalar::shared(&node))
    .filter(DST_ID.eq_shared(&node))
    .filter(REL.eq("co_changed"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"UPDATE "edges" SET "src_id" = ?1 WHERE "edges"."dst_id" = ?1 AND "edges"."rel" = ?2"#
  );
  assert_eq!(params, vec![text("candidate"), text("co_changed")]);
}

#[test]
fn a_delete_shares_a_handle_across_nested_and_or() {
  let node = SharedBind::new("candidate");
  let files = SharedBindList::new(["a", "b"]);

  let (sql, params) = DeleteBuilder::new("edges")
    .filter(
      SRC_ID
        .eq_shared(&node)
        .and(DST_ID.in_shared(&files))
        .or(DST_ID.eq_shared(&node).and(SRC_ID.in_shared(&files))),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"DELETE FROM "edges" WHERE (("edges"."src_id" = ?1 AND "edges"."dst_id" IN (?2, ?3))"#,
      r#" OR ("edges"."dst_id" = ?1 AND "edges"."src_id" IN (?2, ?3)))"#
    )
  );
  assert_eq!(params, vec![text("candidate"), text("a"), text("b")]);
}

#[test]
fn an_insert_select_shares_a_handle_with_its_conflict_assignment() {
  let node = SharedBind::new("candidate");
  let source = SelectBuilder::new("edges")
    .columns_qualified(&[&SRC_ID])
    .filter(DST_ID.eq_shared(&node));

  let (sql, params) = InsertBuilder::new("edges")
    .select(&[&SRC_ID], source)
    .on_conflict(OnConflict::column(&SRC_ID).set_scalar(&DST_ID, Scalar::shared(&node)))
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "edges" ("src_id") SELECT "edges"."src_id" FROM "edges""#,
      r#" WHERE "edges"."dst_id" = $1"#,
      r#" ON CONFLICT ("src_id") DO UPDATE SET "dst_id" = $1"#
    )
  );
  assert_eq!(params, vec![text("candidate")]);
}

#[test]
fn an_insert_values_row_shares_a_handle_between_two_columns() {
  let node = SharedBind::new("candidate");

  let (sql, params) = InsertBuilder::new("edges")
    .set_scalar(&SRC_ID, Scalar::shared(&node))
    .set_scalar(&DST_ID, Scalar::shared(&node))
    .set(&REL, "co_changed")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "edges" ("src_id", "dst_id", "rel") VALUES (?1, ?1, ?2)"#
  );
  assert_eq!(params, vec![text("candidate"), text("co_changed")]);
}
