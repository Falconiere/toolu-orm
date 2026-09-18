//! Conflict target and action: `DO NOTHING`, `DO UPDATE SET`, composite
//! targets, `excluded` references, and which conflict policy wins.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};

use super::columns::{BODY, LAST_USED, MEMORY_ID, PATH, REPO, USED_COUNT, WORKSPACE_ID};

type TestResult = Result<(), DbCoreError>;

#[test]
fn a_clause_without_assignments_renders_do_nothing() {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "new")
    .on_conflict(OnConflict::column(&MEMORY_ID))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "memories" ("id", "body") VALUES (?1, ?2) ON CONFLICT ("id") DO NOTHING"#
  );
  assert_eq!(params.len(), 2);
}

#[test]
fn do_nothing_discards_every_assignment_recorded_before_it() {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set(&BODY, "ignored")
        .set(&LAST_USED, "ignored")
        .do_nothing(),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "memories" ("id") VALUES (?1) ON CONFLICT ("id") DO NOTHING"#
  );
  assert_eq!(params.len(), 1, "a discarded assignment binds nothing");
}

#[test]
fn a_counter_update_reads_the_existing_row_and_binds_after_the_values() {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&USED_COUNT, 1_i64)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_scalar(&USED_COUNT, Scalar::col(&USED_COUNT) + Scalar::bind(1_i64))
        .set(&LAST_USED, "t1"),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "memories" ("id", "used_count") VALUES (?1, ?2) "#,
      r#"ON CONFLICT ("id") DO UPDATE SET "#,
      r#""used_count" = ("memories"."used_count" + ?3), "last_used" = ?4"#
    )
  );
  assert_eq!(
    params,
    vec![
      Value::Text("m1".to_owned()),
      Value::Integer(1),
      Value::Integer(1),
      Value::Text("t1".to_owned()),
    ]
  );
}

#[test]
fn the_same_upsert_renders_with_dollar_placeholders_on_postgres() {
  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&USED_COUNT, 1_i64)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_scalar(&USED_COUNT, Scalar::col(&USED_COUNT) + Scalar::bind(1_i64))
        .set(&LAST_USED, "t1"),
    )
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "memories" ("id", "used_count") VALUES ($1, $2) "#,
      r#"ON CONFLICT ("id") DO UPDATE SET "#,
      r#""used_count" = ("memories"."used_count" + $3), "last_used" = $4"#
    )
  );
  assert_eq!(params.len(), 4);
}

#[test]
fn a_composite_target_lists_its_columns_in_call_order() {
  let (sql, _) = InsertBuilder::new("code_symbols")
    .set(&REPO, "r")
    .set(&PATH, "p")
    .on_conflict(OnConflict::column(&REPO).and_column(&PATH))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "code_symbols" ("repo", "path") VALUES (?1, ?2) ON CONFLICT ("repo", "path") DO NOTHING"#
  );
}

#[test]
fn excluded_and_coalesce_assignments_render_the_upsert_vocabulary() -> TestResult {
  let keep_stored = Scalar::func(
    "coalesce",
    vec![Scalar::col(&WORKSPACE_ID), Scalar::excluded(&WORKSPACE_ID)],
  )?;

  let (sql, params) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "new")
    .set_null(&WORKSPACE_ID)
    .on_conflict(
      OnConflict::column(&MEMORY_ID)
        .set_excluded(&BODY)
        .set_scalar(&WORKSPACE_ID, keep_stored),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    concat!(
      r#"INSERT INTO "memories" ("id", "body", "workspace_id") VALUES (?1, ?2, ?3) "#,
      r#"ON CONFLICT ("id") DO UPDATE SET "body" = "excluded"."body", "#,
      r#""workspace_id" = coalesce("memories"."workspace_id", "excluded"."workspace_id")"#
    )
  );
  assert_eq!(params.len(), 3, "both assignments bind nothing");
  Ok(())
}

#[test]
fn a_later_or_replace_replaces_the_clause_and_a_later_clause_replaces_it() {
  let (replaced, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .on_conflict(OnConflict::column(&MEMORY_ID).set(&BODY, "new"))
    .or_replace()
    .to_sql_for(Dialect::Sqlite);
  assert_eq!(
    replaced, r#"INSERT OR REPLACE INTO "memories" ("id") VALUES (?1)"#,
    "the later or_replace wins"
  );

  let (clause, _) = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .or_replace()
    .conflict_columns(&["ignored"])
    .on_conflict(OnConflict::column(&MEMORY_ID).set(&BODY, "new"))
    .to_sql_for(Dialect::Postgres);
  assert_eq!(
    clause,
    r#"INSERT INTO "memories" ("id") VALUES ($1) ON CONFLICT ("id") DO UPDATE SET "body" = $2"#,
    "the later clause wins and conflict_columns does not reach it"
  );
}
