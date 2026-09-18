//! `TableRef::in_database` — the structured database/schema qualifier issue
//! #114 needs, so `main.indexed_files` renders as two identifiers rather than
//! one name containing a dot.
//!
//! Pure rendering. The executed counterparts, including a real `ATTACH`, live
//! in `toolu-orm-query`'s `rusqlite_insert_select_test` and
//! `postgres_insert_select_test` binaries.
//!
//! Every placeholder assertion names its dialect: this binary is compiled in
//! the default lane *and* the postgres lane, where `Dialect::CURRENT` is
//! Postgres.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::value::Value;

const REPO: Column<Text> = Column::new("indexed_files", "repo");

/// `to_sql_fragment_for` at `start = 1` on `dialect`, dropping the values.
fn render(table: &TableRef, dialect: Dialect) -> String {
  table.to_sql_fragment_for(1, dialect).0
}

#[test]
fn a_qualified_relation_renders_two_identifiers_not_one_dotted_name() {
  let qualified = TableRef::new("indexed_files").in_database("main");
  assert_eq!(
    render(&qualified, Dialect::Sqlite),
    r#""main"."indexed_files""#
  );
  assert_eq!(
    render(&qualified, Dialect::Postgres),
    r#""main"."indexed_files""#,
    "the spelling is identical on both engines; only what it resolves to differs"
  );

  // The bug this exists to prevent: a dotted string is ONE identifier.
  assert_eq!(
    render(&TableRef::new("main.indexed_files"), Dialect::Sqlite),
    r#""main.indexed_files""#,
    "still names a table whose name contains a dot — which is why the parts are separate"
  );
}

#[test]
fn the_qualifier_is_readable_and_the_last_call_wins() {
  let once = TableRef::new("indexed_files").in_database("old");
  assert_eq!(once.database(), Some("old"));
  assert_eq!(once.table(), "indexed_files");

  let twice = once.in_database("main");
  assert_eq!(twice.database(), Some("main"), "a later call replaces it");
  assert_eq!(render(&twice, Dialect::Sqlite), r#""main"."indexed_files""#);

  assert_eq!(TableRef::new("indexed_files").database(), None);
}

#[test]
fn columns_stay_qualified_by_the_alias_or_table_never_by_the_database() {
  let plain = TableRef::new("indexed_files").in_database("old");
  assert_eq!(plain.qualifier(), "indexed_files");
  assert_eq!(plain.column(&REPO).qualified(), r#""indexed_files"."repo""#);

  let aliased = TableRef::aliased("indexed_files", "src").in_database("old");
  assert_eq!(aliased.qualifier(), "src");
  assert_eq!(aliased.column(&REPO).qualified(), r#""src"."repo""#);
  assert_eq!(
    render(&aliased, Dialect::Sqlite),
    r#""old"."indexed_files" AS "src""#,
    "database, then name, then alias"
  );
}

#[test]
fn every_part_doubles_an_embedded_quote() {
  let hostile = TableRef::aliased(r#"in"dexed"#, r#"s"rc"#).in_database(r#"o"ld"#);
  assert_eq!(
    render(&hostile, Dialect::Sqlite),
    r#""o""ld"."in""dexed" AS "s""rc""#,
    "doubling is the only escape a delimited identifier has on either engine"
  );
  assert_eq!(
    render(&hostile, Dialect::Postgres),
    render(&hostile, Dialect::Sqlite)
  );
}

#[test]
fn a_qualified_table_function_keeps_its_arguments_numbered_from_start(
) -> Result<(), toolu_orm_core::error::DbCoreError> {
  let call = TableRef::function(
    "generate_series",
    vec![Value::Integer(1), Value::Integer(9)],
  )?
  .in_database("pg_catalog")
  .with_alias("n");

  let (sql, params) = call.to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, r#""pg_catalog".generate_series($3, $4) AS "n""#);
  assert_eq!(params, vec![Value::Integer(1), Value::Integer(9)]);

  assert_eq!(
    call.to_sql_fragment_for(3, Dialect::Sqlite).0,
    r#""pg_catalog".generate_series(?3, ?4) AS "n""#,
    "rendered verbatim; SQLite has no schema-qualified table function and would reject it, \
     the same caller error as naming a SQLite function on Postgres"
  );
  Ok(())
}

#[test]
fn an_unqualified_table_ref_renders_exactly_what_it_rendered_before() {
  assert_eq!(
    render(&TableRef::new("indexed_files"), Dialect::Sqlite),
    r#""indexed_files""#
  );
  assert_eq!(
    render(&TableRef::aliased("indexed_files", "src"), Dialect::Sqlite),
    r#""indexed_files" AS "src""#
  );
}
