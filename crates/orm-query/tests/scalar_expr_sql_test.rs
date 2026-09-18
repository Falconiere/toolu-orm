//! Scalar expressions in the builders: rendered SQL and the exact parameter
//! order across SELECT, SET and WHERE, on both dialects.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{like_pattern_literal, Scalar};
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps, TextOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;

const ID: Column<Text> = Column::new("memories", "id");
const BODY: Column<Text> = Column::new("memories", "body");
const CREATED_AT: Column<Text> = Column::new("memories", "created_at");
const LAST_SEEN: Column<Text> = Column::new("memories", "last_accessed");
const HITS: Column<Integer> = Column::new("memories", "access_count");

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// `coalesce("last_accessed", "created_at")`.
fn effective_time() -> Result<Scalar, toolu_orm_core::error::DbCoreError> {
  Scalar::func(
    "coalesce",
    vec![Scalar::col(&LAST_SEEN), Scalar::col(&CREATED_AT)],
  )
}

// ── SELECT: projection, filter and ordering binds in one statement ───────────

#[test]
fn select_numbers_projection_then_filter_then_order_on_sqlite() -> TestResult {
  let (sql, params) = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(
      Scalar::case_when(HITS.gt(0), Scalar::bind("hot")).otherwise(Scalar::bind("cold")),
      "heat",
    )
    .filter(BODY.like_escape(format!("%{}%", like_pattern_literal("100%", '\\')), '\\'))
    .order_by(effective_time()?.desc())
    .limit(5)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "id", CASE WHEN "memories"."access_count" > ?1 THEN ?2 ELSE ?3 END AS "heat" "#
      .to_owned()
      + r#"FROM "memories" WHERE "memories"."body" LIKE ?4 ESCAPE ?5 "#
      + r#"ORDER BY coalesce("memories"."last_accessed", "memories"."created_at") DESC LIMIT ?6"#
  );
  assert_eq!(
    params,
    vec![
      Value::Integer(0),
      text("hot"),
      text("cold"),
      text(r"%100\%%"),
      text("\\"),
      Value::Integer(5),
    ]
  );
  Ok(())
}

#[test]
fn select_numbers_the_same_statement_with_dollar_placeholders_on_postgres() -> TestResult {
  let (sql, params) = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(
      Scalar::case_when(HITS.gt(0), Scalar::bind("hot")).otherwise(Scalar::bind("cold")),
      "heat",
    )
    .filter(BODY.like_escape("%x%", '\\'))
    .order_by(effective_time()?.desc())
    .to_sql_for(Dialect::Postgres);

  assert!(
    sql.contains(r#"CASE WHEN "memories"."access_count" > $1 THEN $2 ELSE $3 END AS "heat""#),
    "unexpected projection: {sql}"
  );
  assert!(
    sql.contains(r#"WHERE "memories"."body" LIKE $4 ESCAPE $5"#),
    "unexpected filter: {sql}"
  );
  assert_eq!(params.len(), 5);
  Ok(())
}

#[test]
fn an_order_term_that_binds_is_numbered_after_the_filter() {
  let (sql, params) = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .filter(ID.eq("m1"))
    .order_by(
      Scalar::case_when(HITS.gt(3), Scalar::bind(0))
        .otherwise(Scalar::bind(1))
        .asc(),
    )
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.ends_with(r#"ORDER BY CASE WHEN "memories"."access_count" > ?2 THEN ?3 ELSE ?4 END ASC"#),
    "unexpected order clause: {sql}"
  );
  assert_eq!(
    params,
    vec![
      text("m1"),
      Value::Integer(3),
      Value::Integer(0),
      Value::Integer(1)
    ]
  );
}

#[test]
fn column_expr_and_column_scalar_render_in_call_order() {
  let (sql, params) = SelectBuilder::new("memories")
    .column_expr("1", "one")
    .column_scalar(Scalar::bind("two"), "two")
    .column_expr("3", "three")
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT 1 AS "one", ?1 AS "two", 3 AS "three" FROM "memories""#
  );
  assert_eq!(params, vec![text("two")]);
}

#[test]
fn count_drops_a_bound_projection_and_numbers_the_filter_from_one() {
  let builder = SelectBuilder::new("memories")
    .columns_raw(&["id"])
    .column_scalar(Scalar::bind("ignored"), "label")
    .filter(HITS.gt(2));

  let (count_sql, count_params) = builder.to_count_sql_for(Dialect::Sqlite);
  assert_eq!(
    count_sql,
    r#"SELECT COUNT(*) FROM "memories" WHERE "memories"."access_count" > ?1"#
  );
  assert_eq!(count_params, vec![Value::Integer(2)]);

  let (exists_sql, exists_params) = builder.to_exists_sql_for(Dialect::Sqlite);
  assert!(
    exists_sql.contains(r#""memories"."access_count" > ?1"#),
    "unexpected exists SQL: {exists_sql}"
  );
  assert_eq!(exists_params.len(), 1);
}

// ── UPDATE: SET binds before WHERE binds ────────────────────────────────────

#[test]
fn update_numbers_set_scalars_before_the_filter() {
  let (sql, params) = UpdateBuilder::new("memories")
    .set_scalar(&HITS, Scalar::col(&HITS) + Scalar::bind(1))
    .set(&LAST_SEEN, "2026-09-18T11:00:00Z")
    .filter(ID.eq("m1"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"UPDATE "memories" SET "access_count" = ("memories"."access_count" + ?1), "#.to_owned()
      + r#""last_accessed" = ?2 WHERE "memories"."id" = ?3"#
  );
  assert_eq!(
    params,
    vec![Value::Integer(1), text("2026-09-18T11:00:00Z"), text("m1")]
  );
}

#[test]
fn update_keeps_the_same_order_on_postgres() {
  let (sql, params) = UpdateBuilder::new("memories")
    .set_scalar(&HITS, Scalar::col(&HITS) + Scalar::bind(1))
    .filter(ID.eq("m1"))
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"UPDATE "memories" SET "access_count" = ("memories"."access_count" + $1) "#.to_owned()
      + r#"WHERE "memories"."id" = $2"#
  );
  assert_eq!(params, vec![Value::Integer(1), text("m1")]);
}

#[test]
fn set_expr_still_renders_raw_text_without_a_parameter() {
  let (sql, params) = UpdateBuilder::new("memories")
    .set_expr(&HITS, r#""access_count" + 1"#)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"UPDATE "memories" SET "access_count" = "access_count" + 1"#
  );
  assert!(params.is_empty());
}

// ── INSERT: scalars in the VALUES list ──────────────────────────────────────

#[test]
fn insert_mixes_bound_values_with_a_computed_one() -> TestResult {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&ID, "m1")
    .set_scalar(
      &CREATED_AT,
      Scalar::func("datetime", vec![Scalar::bind("2026-09-18T10:00:00Z")])?,
    )
    .set_scalar(&HITS, Scalar::bind(1) + Scalar::bind(2))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "memories" ("id", "created_at", "access_count") "#.to_owned()
      + r#"VALUES (?1, datetime(?2), (?3 + ?4))"#
  );
  assert_eq!(
    params,
    vec![
      text("m1"),
      text("2026-09-18T10:00:00Z"),
      Value::Integer(1),
      Value::Integer(2)
    ]
  );
  Ok(())
}
