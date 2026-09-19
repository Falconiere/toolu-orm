//! What the `DISTINCT`, `GROUP BY` and `HAVING` clauses render.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::fixtures::{
  distinct_paths, files, sources, status_counts, LABEL, PATH, SIZE_BYTES, SOURCE_ID, SOURCE_PK,
  STATUS,
};

#[test]
fn a_grouped_report_renders_the_issues_statement() {
  let (sql, params) = status_counts().to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "status", COUNT(*) AS "n" FROM "source_files" "#.to_owned()
      + r#"WHERE "source_files"."source_id" = ?1 GROUP BY "source_files"."status""#
  );
  assert_eq!(params, vec![Value::Text("s1".to_owned())]);
}

#[test]
fn distinct_renders_before_the_select_list_and_after_it_the_page() {
  let (sql, params) = distinct_paths()
    .limit(2)
    .offset(1)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT DISTINCT "source_files"."path" FROM "source_files" "#.to_owned()
      + r#"WHERE "source_files"."source_id" = ?1 "#
      + r#"ORDER BY "source_files"."path" ASC LIMIT ?2 OFFSET ?3"#
  );
  assert_eq!(
    params,
    vec![
      Value::Text("s1".to_owned()),
      Value::Integer(2),
      Value::Integer(1),
    ]
  );
}

#[test]
fn several_grouping_terms_render_in_call_order() {
  let (sql, _) = SelectBuilder::new("source_files")
    .columns_raw(&["source_id", "status"])
    .column_scalar(Scalar::count_star(), "n")
    .group_by(&SOURCE_ID)
    .group_by(&STATUS)
    .to_sql_for(Dialect::Sqlite);

  assert!(
    sql.ends_with(r#"GROUP BY "source_files"."source_id", "source_files"."status""#),
    "unexpected tail: {sql}"
  );
}

#[test]
fn every_aggregate_projection_renders_in_one_select_list() {
  let (sql, params) = SelectBuilder::new("source_files")
    .columns_raw(&["source_id"])
    .column_scalar(Scalar::count_star(), "rows")
    .column_scalar(Scalar::count_distinct(Scalar::col(&PATH)), "paths")
    .column_scalar(Scalar::sum(Scalar::col(&SIZE_BYTES)), "total")
    .column_scalar(Scalar::max(Scalar::col(&SIZE_BYTES)), "largest")
    .column_scalar(Scalar::min(Scalar::col(&SIZE_BYTES)), "smallest")
    .column_scalar(Scalar::avg(Scalar::col(&SIZE_BYTES)), "mean")
    .group_by(&SOURCE_ID)
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "source_id", COUNT(*) AS "rows", "#.to_owned()
      + r#"COUNT(DISTINCT "source_files"."path") AS "paths", "#
      + r#"SUM("source_files"."size_bytes") AS "total", "#
      + r#"MAX("source_files"."size_bytes") AS "largest", "#
      + r#"MIN("source_files"."size_bytes") AS "smallest", "#
      + r#"AVG("source_files"."size_bytes") AS "mean" "#
      + r#"FROM "source_files" GROUP BY "source_files"."source_id""#
  );
  assert!(params.is_empty(), "aggregates over columns bind nothing");
}

#[test]
fn several_having_conjuncts_join_with_and() {
  let (sql, params) = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .having(Scalar::sum(Scalar::col(&SIZE_BYTES)).lt(Scalar::bind(500i64)))
    .to_sql_for(Dialect::Sqlite);

  assert!(sql.ends_with(r#"HAVING COUNT(*) > ?2 AND SUM("source_files"."size_bytes") < ?3"#));
  assert_eq!(params.len(), 3);
}

/// The raw escape hatch keeps working beside the typed aggregates, in call
/// order, and a raw grouping term is still reachable.
#[test]
fn the_raw_escape_hatch_still_projects_and_groups() {
  let (sql, _) = SelectBuilder::new("source_files")
    .column_expr("1 + 1", "two")
    .column_scalar(Scalar::count_star(), "n")
    .group_by_scalar(Scalar::sql(r#""source_files"."status""#))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT 1 + 1 AS "two", COUNT(*) AS "n" FROM "source_files" "#.to_owned()
      + r#"GROUP BY "source_files"."status""#
  );
}

/// A grouping key from the *joined* relation, under its alias.
#[test]
fn a_qualified_grouping_key_names_the_joined_alias() {
  let f = files();
  let s = sources();

  let (sql, _) = SelectBuilder::from_table(&f)
    .column_as(&s.column(&LABEL), "label")
    .column_scalar(Scalar::count_star(), "n")
    .join(&s, s.column(&SOURCE_PK).equals(&f.column(&SOURCE_ID)))
    .group_by(&s.column(&LABEL))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "s"."label" AS "label", COUNT(*) AS "n" FROM "source_files" AS "f" "#.to_owned()
      + r#"INNER JOIN "sources" AS "s" ON "s"."id" = "f"."source_id" "#
      + r#"GROUP BY "s"."label""#
  );
}

/// `distinct()` sets a flag, so saying it twice says it once.
#[test]
fn distinct_is_idempotent() {
  let once = distinct_paths().to_sql_for(Dialect::Sqlite).0;
  let twice = distinct_paths().distinct().to_sql_for(Dialect::Sqlite).0;

  assert_eq!(once, twice);
  assert_eq!(once.matches("DISTINCT").count(), 1);
}
