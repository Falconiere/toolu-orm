//! One statement whose `SELECT` source binds *and* whose conflict clause
//! binds, pinned as an exact `(sql, params)` pair on both dialects.
//!
//! This is the part that corrupts data silently when it is wrong: a value
//! lands under the wrong placeholder and no engine complains. So the whole
//! rendering is asserted, both dialects, plus a programmatic check that the
//! indices run `1..=params.len()` with no repeat and no gap.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{placeholder_indices, source, target, text, INDEXED_AT, PATH, RANK, REPO};

/// A source binding a projected default (`?1`) and a `WHERE` value (`?2`),
/// under a conflict clause binding a third (`?3`).
fn copy_with_binds_on_both_sides() -> InsertBuilder {
  let older = SelectBuilder::from_table(source())
    .columns_raw(&["repo", "path"])
    .column_scalar(Scalar::bind("legacy"), "rank")
    .filter(REPO.eq("r1"));

  InsertBuilder::into_table(target())
    .select(&[&REPO, &PATH, &RANK], older)
    .on_conflict(OnConflict::column(&REPO).set(&INDEXED_AT, 99_i64))
}

#[test]
fn source_binds_precede_conflict_binds_on_sqlite() {
  let (sql, params) = copy_with_binds_on_both_sides().to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path", "rank") SELECT * FROM (SELECT "repo", "path", ?1 AS "rank" FROM "old"."indexed_files" WHERE "indexed_files"."repo" = ?2) AS "toolu_insert_source" WHERE true ON CONFLICT ("repo") DO UPDATE SET "indexed_at" = ?3"#
  );
  assert_eq!(
    params,
    vec![text("legacy"), text("r1"), Value::Integer(99)],
    "projection, then WHERE, then the conflict clause — the order they were written"
  );
}

#[test]
fn source_binds_precede_conflict_binds_on_postgres() {
  let (sql, params) = copy_with_binds_on_both_sides().to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path", "rank") SELECT "repo", "path", $1 AS "rank" FROM "old"."indexed_files" WHERE "indexed_files"."repo" = $2 ON CONFLICT ("repo") DO UPDATE SET "indexed_at" = $3"#
  );
  assert_eq!(params, vec![text("legacy"), text("r1"), Value::Integer(99)]);
}

#[test]
fn the_indices_run_one_through_len_with_no_repeat_and_no_gap() {
  for dialect in [Dialect::Sqlite, Dialect::Postgres] {
    let (sql, params) = copy_with_binds_on_both_sides().to_sql_for(dialect);
    let indices = placeholder_indices(&sql);

    assert_eq!(
      indices.len(),
      params.len(),
      "one placeholder per bound value, {dialect:?}: {sql}"
    );
    let expected: Vec<usize> = (1..=params.len()).collect();
    assert_eq!(indices, expected, "{dialect:?}: {sql}");
  }
}

#[test]
fn a_zero_bind_source_leaves_the_conflict_clause_starting_at_one() {
  let plain = SelectBuilder::from_table(source()).columns_raw(&["repo"]);
  let (sql, params) = InsertBuilder::into_table(target())
    .select(&[&REPO], plain)
    .on_conflict(OnConflict::column(&REPO).set(&INDEXED_AT, 7_i64))
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.ends_with(r#"ON CONFLICT ("repo") DO UPDATE SET "indexed_at" = ?1"#),
    "nothing preceded it, so the clause takes the first index: {sql}"
  );
  assert_eq!(params, vec![Value::Integer(7)]);
}

#[test]
fn the_guard_wrapper_itself_consumes_no_index() {
  let guarded = copy_with_binds_on_both_sides().to_sql_for(Dialect::Sqlite);
  let unguarded = copy_with_binds_on_both_sides().to_sql_for(Dialect::Postgres);

  assert_eq!(
    placeholder_indices(&guarded.0),
    placeholder_indices(&unguarded.0),
    "wrapping the source in a derived table binds nothing, so numbering is identical"
  );
  assert_eq!(guarded.1, unguarded.1);
}
