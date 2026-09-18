//! AC-10 and AC-11: a grouping key from a joined, aliased relation, and the
//! raw `column_expr` escape hatch still working beside the typed aggregates.

use toolu_orm_core::expr::{OrderBy, Scalar};
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, pairs, KeyCount};
use crate::seed::{LABEL, PATH, SOURCE_ID, SOURCE_PK};
use crate::support::{files, sources, TestResult};

/// The grouping key comes from `sources AS "s"` — a table reached through a
/// join, under an alias that hides its real name. `s1` owns six rows and `s2`
/// one, so the labels carry those counts.
#[tokio::test]
async fn grouping_by_a_joined_aliased_column_counts_per_label() -> TestResult {
  let conn = db::setup_db().await?;
  let f = files();
  let s = sources();

  let rows: Vec<KeyCount> = SelectBuilder::from_table(&f)
    .column_as(&s.column(&LABEL), "label")
    .column_scalar(Scalar::count_star(), "n")
    .join(&s, s.column(&SOURCE_PK).equals(&f.column(&SOURCE_ID)))
    .group_by(&s.column(&LABEL))
    .order_by(s.column(&LABEL).asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(pairs(&rows), vec![("primary", 6), ("secondary", 1)]);
  Ok(())
}

/// An aggregate over a column of the *aliased* relation, alongside a grouping
/// key from the joined one.
#[tokio::test]
async fn an_aggregate_can_read_a_column_of_the_aliased_relation() -> TestResult {
  let conn = db::setup_db().await?;
  let f = files();
  let s = sources();

  let rows: Vec<KeyCount> = SelectBuilder::from_table(&f)
    .column_as(&s.column(&LABEL), "label")
    .column_scalar(Scalar::count_distinct(f.column(&PATH).scalar()), "n")
    .join(&s, s.column(&SOURCE_PK).equals(&f.column(&SOURCE_ID)))
    .group_by(&s.column(&LABEL))
    .order_by(s.column(&LABEL).asc())
    .fetch_all(&conn)
    .await?;

  // s1 spans three distinct paths, s2 one.
  assert_eq!(pairs(&rows), vec![("primary", 3), ("secondary", 1)]);
  Ok(())
}

/// AC-11: `column_expr` takes raw SQL that binds nothing and still projects in
/// call order beside a typed aggregate.
#[tokio::test]
async fn the_raw_column_expr_escape_hatch_still_projects() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .column_expr("'total'", "key")
    .column_scalar(Scalar::count_star(), "n")
    .fetch_all(&conn)
    .await?;

  assert_eq!(pairs(&rows), vec![("total", 7)]);
  Ok(())
}

/// `group_by_scalar` accepts a raw term, which is the grouping-side escape
/// hatch: grouping on a path prefix rather than on a column.
#[tokio::test]
async fn a_raw_grouping_term_groups_by_a_computed_key() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .column_expr(r#"substr("path", 1, 5)"#, "key")
    .column_scalar(Scalar::count_star(), "n")
    .group_by_scalar(Scalar::sql(r#"substr("path", 1, 5)"#))
    .order_by(OrderBy::alias_asc("key"))
    .fetch_all(&conn)
    .await?;

  // The seven rows carry three five-character prefixes: src/a (f1, f2, f5),
  // src/b (f3, f6) and src/c (f4, f7).
  assert_eq!(pairs(&rows), vec![("src/a", 3), ("src/b", 2), ("src/c", 2)]);
  Ok(())
}
