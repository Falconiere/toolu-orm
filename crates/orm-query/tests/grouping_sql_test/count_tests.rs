//! `to_count_sql_for` and `to_exists_sql_for` once grouping is in play.
//!
//! The decision under test: a grouped or distinct `count()` reports **how many
//! rows the unpaginated query returns** — groups, or distinct rows — which
//! needs a derived table. Appending `GROUP BY` to a bare `SELECT COUNT(*)`
//! would instead return one row per group.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{distinct_paths, status_counts, SOURCE_ID, STATUS};

/// The regression guard for every existing consumer: a builder using none of
/// the new clauses renders the count it always rendered.
#[test]
fn an_ungrouped_count_renders_the_plain_form_unchanged() {
  let (sql, params) = SelectBuilder::new("source_files")
    .columns_raw(&["id"])
    .filter(SOURCE_ID.eq("s1"))
    .order_by(STATUS.asc())
    .limit(10)
    .to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM "source_files" WHERE "source_files"."source_id" = ?1"#
  );
  assert_eq!(params, vec![Value::Text("s1".to_owned())]);
}

#[test]
fn a_grouped_count_wraps_the_statement_in_an_aliased_derived_table() {
  let (sql, params) = status_counts().to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM (SELECT "status", COUNT(*) AS "n" FROM "source_files" "#.to_owned()
      + r#"WHERE "source_files"."source_id" = ?1 GROUP BY "source_files"."status") "#
      + r#"AS "toolu_count""#
  );
  assert_eq!(params, vec![Value::Text("s1".to_owned())]);
}

/// Postgres *requires* an alias on a subquery in `FROM`; the SQLite form
/// carries the same one, so there is a single rendering to reason about.
#[test]
fn the_derived_table_is_aliased_on_postgres_too() {
  let (sql, _) = status_counts().to_count_sql_for(Dialect::Postgres);

  assert!(
    sql.starts_with("SELECT COUNT(*) FROM (SELECT "),
    "got: {sql}"
  );
  assert!(sql.ends_with(r#") AS "toolu_count""#), "got: {sql}");
  assert!(sql.contains("= $1"), "got: {sql}");
}

#[test]
fn a_distinct_count_wraps_so_it_counts_distinct_rows() {
  let (sql, _) = distinct_paths().to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM (SELECT DISTINCT "source_files"."path" FROM "source_files" "#
      .to_owned()
      + r#"WHERE "source_files"."source_id" = ?1) AS "toolu_count""#
  );
}

/// `ORDER BY` and the page are dropped from the inner statement, so the count
/// is of the whole result and its own numbering starts at `?1`.
#[test]
fn the_counted_inner_statement_drops_ordering_and_pagination() {
  let (sql, params) = distinct_paths()
    .limit(2)
    .offset(1)
    .to_count_sql_for(Dialect::Sqlite);

  assert!(!sql.contains("ORDER BY"), "got: {sql}");
  assert!(!sql.contains("LIMIT"), "got: {sql}");
  assert!(!sql.contains("OFFSET"), "got: {sql}");
  assert_eq!(params, vec![Value::Text("s1".to_owned())]);
}

#[test]
fn a_having_clause_participates_in_the_count() {
  let (sql, params) = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .to_count_sql_for(Dialect::Sqlite);

  assert!(sql.contains("HAVING COUNT(*) > ?2"), "got: {sql}");
  assert_eq!(
    params,
    vec![Value::Text("s1".to_owned()), Value::Integer(1)]
  );
}

#[test]
fn exists_carries_the_grouping_but_not_distinct() {
  let (sql, params) = status_counts()
    .distinct()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .to_exists_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT EXISTS(SELECT 1 FROM "source_files" "#.to_owned()
      + r#"WHERE "source_files"."source_id" = ?1 "#
      + r#"GROUP BY "source_files"."status" HAVING COUNT(*) > ?2)"#
  );
  assert_eq!(
    params,
    vec![Value::Text("s1".to_owned()), Value::Integer(1)]
  );
}

/// An ungrouped `exists()` is untouched by this change.
#[test]
fn an_ungrouped_exists_renders_the_plain_form_unchanged() {
  let (sql, _) = SelectBuilder::new("source_files")
    .filter(SOURCE_ID.eq("s1"))
    .to_exists_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT EXISTS(SELECT 1 FROM "source_files" WHERE "source_files"."source_id" = ?1)"#
  );
}
