//! `DISTINCT`, `GROUP BY`, `HAVING` and the aggregate constructors compose
//! with `toolu-orm` as the only dependency.
//!
//! Like its sibling binaries this package depends on `toolu-orm` alone, so
//! `toolu_orm_core` is absent from the extern prelude and every path here is
//! written through the facade.

use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::expr::{OrderBy, Scalar};
use toolu_orm::table;

#[table(name = "facade_grouping_rows")]
pub struct FacadeGroupingRow {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub status: Text,
  pub size_bytes: Integer,
}

/// Rendered for an explicit dialect: `to_sql()` follows `Dialect::CURRENT`,
/// which is Postgres (`$N`) in the postgres lane and SQLite (`?N`) elsewhere,
/// and this asserts the placeholder text.
#[test]
fn a_grouped_report_composes_through_the_facade() {
  let (sql, params) = FacadeGroupingRow::select()
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .column_scalar(
      Scalar::sum(Scalar::col(&facade_grouping_rows::size_bytes)),
      "total",
    )
    .group_by(&facade_grouping_rows::status)
    .having(Scalar::count_star().gt(Scalar::bind(1)))
    .order_by(OrderBy::alias_desc("n"))
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "status", COUNT(*) AS "n", "#.to_owned()
      + r#"SUM("facade_grouping_rows"."size_bytes") AS "total" "#
      + r#"FROM "facade_grouping_rows" "#
      + r#"GROUP BY "facade_grouping_rows"."status" "#
      + r#"HAVING COUNT(*) > ?1 ORDER BY "n" DESC"#
  );
  assert_eq!(params.len(), 1);
}

/// The grouped-count contract is reachable through the facade too: a grouped
/// count reads a derived table, so it reports the number of groups.
#[test]
fn a_grouped_count_wraps_a_derived_table_through_the_facade() {
  let (sql, _) = FacadeGroupingRow::select()
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .group_by(&facade_grouping_rows::status)
    .to_count_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT COUNT(*) FROM (SELECT "status", COUNT(*) AS "n" "#.to_owned()
      + r#"FROM "facade_grouping_rows" GROUP BY "facade_grouping_rows"."status") "#
      + r#"AS "toolu_count""#
  );
}

#[test]
fn a_distinct_listing_composes_through_the_facade() {
  let (sql, _) = FacadeGroupingRow::select()
    .columns_qualified(&[&facade_grouping_rows::status])
    .distinct()
    .order_by(facade_grouping_rows::status.asc())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT DISTINCT "facade_grouping_rows"."status" FROM "facade_grouping_rows" "#.to_owned()
      + r#"ORDER BY "facade_grouping_rows"."status" ASC"#
  );
}
