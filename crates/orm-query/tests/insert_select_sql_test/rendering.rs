//! The statement shape: a database-qualified target, an explicit target column
//! list, and the source appended unparenthesised.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::{Cte, SelectBuilder};

use crate::fixtures::{
  plain_projection, source, target, text, BLOB_OID, INDEXED_AT, PATH, RANK, REPO,
};

/// Issue #114's statement, verbatim, as a builder.
fn copy() -> InsertBuilder {
  InsertBuilder::into_table(target())
    .or_ignore()
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT], plain_projection())
}

#[test]
fn the_issues_copy_renders_a_two_part_target_and_a_bare_select() {
  let (sql, params) = copy().to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT OR IGNORE INTO "main"."indexed_files" ("repo", "path", "blob_oid", "indexed_at") SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files""#
  );
  assert!(params.is_empty(), "this copy binds nothing");

  // The qualifier is two identifiers on both sides, never one dotted name.
  assert!(!sql.contains(r#""main.indexed_files""#));
  assert!(!sql.contains(r#""old.indexed_files""#));
}

#[test]
fn postgres_renders_the_conflict_mode_as_a_clause_and_keeps_the_qualifier() {
  let (sql, params) = copy().to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path", "blob_oid", "indexed_at") SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files" ON CONFLICT DO NOTHING"#
  );
  assert!(params.is_empty());
}

#[test]
fn a_projected_default_fills_a_column_the_source_does_not_have() {
  let older = SelectBuilder::from_table(source())
    .columns_raw(&["repo", "path"])
    .column_scalar(Scalar::bind(0_i64), "rank");

  let (sql, params) = InsertBuilder::into_table(target())
    .or_ignore()
    .select(&[&REPO, &PATH, &RANK], older)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT OR IGNORE INTO "main"."indexed_files" ("repo", "path", "rank") SELECT "repo", "path", ?1 AS "rank" FROM "old"."indexed_files""#
  );
  assert_eq!(params, vec![toolu_orm_core::value::Value::Integer(0)]);
}

#[test]
fn an_empty_target_column_list_renders_no_empty_parentheses() {
  let (sql, _) = InsertBuilder::into_table(target())
    .select(&[], plain_projection())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files""#
  );
  assert!(
    !sql.contains("()"),
    "an empty () is a syntax error on both engines"
  );
}

#[test]
fn select_replaces_any_recorded_values_whichever_order_they_were_called_in() {
  let before = InsertBuilder::into_table(target())
    .set(&REPO, "ignored")
    .select(
      &[&REPO],
      SelectBuilder::from_table(source()).columns_raw(&["repo"]),
    );
  let after = InsertBuilder::into_table(target())
    .select(
      &[&REPO],
      SelectBuilder::from_table(source()).columns_raw(&["repo"]),
    )
    .set(&PATH, "ignored too");

  let expected =
    r#"INSERT INTO "main"."indexed_files" ("repo") SELECT "repo" FROM "old"."indexed_files""#;
  assert_eq!(before.to_sql_for(Dialect::Sqlite).0, expected);
  assert_eq!(after.to_sql_for(Dialect::Sqlite).0, expected);
  assert!(
    before.to_sql_for(Dialect::Sqlite).1.is_empty(),
    "the dropped VALUES bind nothing into the statement"
  );
}

#[test]
fn select_raw_names_the_target_columns_as_strings() {
  let (sql, _) = InsertBuilder::into_table(target())
    .select_raw(
      &["repo", "path"],
      SelectBuilder::from_table(source()).columns_raw(&["repo", "path"]),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path") SELECT "repo", "path" FROM "old"."indexed_files""#
  );
}

#[test]
fn a_with_prefix_and_a_union_arm_are_appended_whole() {
  let recent = SelectBuilder::from_table(source())
    .columns_raw(&["repo"])
    .filter(REPO.eq("r1"));
  let composed = SelectBuilder::new("recent")
    .columns_raw(&["repo"])
    .with(Cte::new("recent", recent))
    .union_all(
      SelectBuilder::from_table(source())
        .columns_raw(&["repo"])
        .filter(REPO.eq("r2")),
    );

  let (sql, params) = InsertBuilder::into_table(target())
    .select(&[&REPO], composed)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo") WITH "recent" AS (SELECT "repo" FROM "old"."indexed_files" WHERE "indexed_files"."repo" = ?1) SELECT "repo" FROM "recent" UNION ALL SELECT "repo" FROM "old"."indexed_files" WHERE "indexed_files"."repo" = ?2"#,
    "the WITH prefix leads the appended statement and the arm continues its numbering"
  );
  assert_eq!(params, vec![text("r1"), text("r2")]);
  assert!(
    !sql.contains("((SELECT"),
    "the source is appended unparenthesised; SQLite rejects a parenthesised insert source"
  );
}

#[test]
fn the_target_renders_its_database_and_table_and_drops_any_alias() {
  let aliased =
    toolu_orm_core::alias::TableRef::aliased("indexed_files", "tgt").in_database("main");
  assert_eq!(
    aliased.qualifier(),
    "tgt",
    "the alias is still on the TableRef"
  );

  let (sql, _) = InsertBuilder::into_table(&aliased)
    .select(
      &[&REPO],
      SelectBuilder::from_table(source()).columns_raw(&["repo"]),
    )
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql, r#"INSERT INTO "main"."indexed_files" ("repo") SELECT "repo" FROM "old"."indexed_files""#,
    "an INSERT target keeps its database and drops its alias, because OnConflict's \
     Scalar::col renders \"table\".\"column\" from the column's own table"
  );
  assert!(!sql.contains(r#"AS "tgt""#));
}

/// The base name an error message carries, never the qualifier or the alias.
#[test]
fn table_name_reports_the_bare_table_whatever_the_target_carries() {
  let qualified =
    toolu_orm_core::alias::TableRef::aliased("indexed_files", "tgt").in_database("main");
  assert_eq!(
    InsertBuilder::into_table(&qualified).table_name(),
    "indexed_files"
  );
  assert_eq!(
    InsertBuilder::new("indexed_files").table_name(),
    "indexed_files"
  );
}

#[test]
fn an_identifier_carrying_a_quote_doubles_it_in_every_part() {
  let (sql, _) = InsertBuilder::into_table(
    toolu_orm_core::alias::TableRef::new(r#"in"dexed"#).in_database(r#"ma"in"#),
  )
  .select_raw(
    &[r#"re"po"#],
    SelectBuilder::new("src").columns_raw(&["repo"]),
  )
  .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "ma""in"."in""dexed" ("re""po") SELECT "repo" FROM "src""#
  );
}
