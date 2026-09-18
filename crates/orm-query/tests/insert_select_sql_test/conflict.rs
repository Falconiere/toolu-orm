//! The conflict modes over a `SELECT` source, and the SQLite-only guard the
//! explicit clause needs.
//!
//! SQLite's parser reads the `ON` of `ON CONFLICT` as a join's `ON` when the
//! insert source is a `SELECT` with a `FROM`:
//!
//! ```text
//! sqlite> INSERT INTO "u" ("a","c") SELECT "a","c" FROM "s" ON CONFLICT ("a") DO UPDATE SET "c" = 1;
//! Parse error: near "DO": syntax error
//! ```
//!
//! So the SQLite rendering wraps the source in a derived table carrying the
//! `WHERE` that SQLite's own documentation prescribes. Postgres parses every
//! one of those shapes unwrapped and is therefore never wrapped — asserted in
//! both directions here, so a regression either way fails an assertion rather
//! than a database.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{plain_projection, source, target, BLOB_OID, INDEXED_AT, PATH, REPO};

const GUARD: &str = r#"AS "toolu_insert_source" WHERE true"#;

fn upsert() -> InsertBuilder {
  InsertBuilder::into_table(target())
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT], plain_projection())
    .on_conflict(
      OnConflict::column(&REPO)
        .and_column(&PATH)
        .set_excluded(&BLOB_OID),
    )
}

#[test]
fn sqlite_wraps_the_source_so_on_conflict_cannot_be_read_as_a_join() {
  let (sql, params) = upsert().to_sql_for(Dialect::Sqlite);
  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path", "blob_oid", "indexed_at") SELECT * FROM (SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files") AS "toolu_insert_source" WHERE true ON CONFLICT ("repo", "path") DO UPDATE SET "blob_oid" = "excluded"."blob_oid""#
  );
  assert!(params.is_empty());
  assert!(sql.contains(GUARD));
}

#[test]
fn postgres_needs_no_guard_and_does_not_get_one() {
  let (sql, params) = upsert().to_sql_for(Dialect::Postgres);
  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path", "blob_oid", "indexed_at") SELECT "repo", "path", "blob_oid", "indexed_at" FROM "old"."indexed_files" ON CONFLICT ("repo", "path") DO UPDATE SET "blob_oid" = "excluded"."blob_oid""#
  );
  assert!(params.is_empty());
  assert!(
    !sql.contains(GUARD),
    "Postgres 16 parses the unwrapped form, including over a source carrying its own JOIN … ON"
  );
}

#[test]
fn the_keyword_conflict_modes_are_never_guarded_on_either_dialect() {
  for builder in [
    InsertBuilder::into_table(target()).or_ignore(),
    InsertBuilder::into_table(target()).or_replace(),
  ] {
    let built = builder.select(
      &[&REPO],
      SelectBuilder::from_table(source()).columns_raw(&["repo"]),
    );
    for dialect in [Dialect::Sqlite, Dialect::Postgres] {
      let (sql, _) = built.to_sql_for(dialect);
      assert!(
        !sql.contains(GUARD),
        "INSERT OR … is a keyword on SQLite, so no trailing ON exists to confuse: {sql}"
      );
    }
  }
}

#[test]
fn a_values_insert_with_an_explicit_clause_is_unchanged_by_the_guard() {
  let (sql, _) = InsertBuilder::into_table(target())
    .set(&REPO, "r1")
    .on_conflict(OnConflict::column(&REPO).do_nothing())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo") VALUES (?1) ON CONFLICT ("repo") DO NOTHING"#,
    "the ambiguity needs a SELECT source; a VALUES list has no FROM to be mistaken for a join"
  );
}

#[test]
fn the_legacy_postgres_replace_targets_the_select_column_list() {
  let (sql, _) = InsertBuilder::into_table(target())
    .or_replace()
    .conflict_columns(&["repo"])
    .select(
      &[&REPO, &PATH],
      SelectBuilder::from_table(source()).columns_raw(&["repo", "path"]),
    )
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"INSERT INTO "main"."indexed_files" ("repo", "path") SELECT "repo", "path" FROM "old"."indexed_files" ON CONFLICT ("repo") DO UPDATE SET "path" = EXCLUDED."path""#,
    "the shorthand reads the columns the SELECT fills, not the ones set() recorded"
  );
}

#[test]
fn returning_renders_after_the_source_and_binds_nothing() {
  let (sql, params) = InsertBuilder::into_table(target())
    .or_ignore()
    .select(
      &[&REPO],
      SelectBuilder::from_table(source()).columns_raw(&["repo"]),
    )
    .returning(&REPO)
    .returning(&PATH)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"INSERT OR IGNORE INTO "main"."indexed_files" ("repo") SELECT "repo" FROM "old"."indexed_files" RETURNING "repo", "path""#
  );
  assert!(params.is_empty());
}
