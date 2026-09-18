//! AC-2 and AC-4: one row per group, over one or several keys, ordered by an
//! aggregate's output alias.

use toolu_orm_core::expr::{OrderBy, Scalar};
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, pairs, KeyCount, PairCount};
use crate::seed::{SOURCE_ID, STATUS};
use crate::support::{status_counts, TestResult};

fn pair(source_id: &str, status: &str, n: i64) -> PairCount {
  PairCount {
    source_id: source_id.to_owned(),
    status: status.to_owned(),
    n,
  }
}

/// The statement from the issue: `SELECT status, COUNT(*) … GROUP BY status`.
#[test]
fn grouping_by_one_column_returns_one_row_per_status() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<KeyCount> = status_counts().order_by(STATUS.asc()).fetch_all(&conn)?;

  assert_eq!(
    pairs(&rows),
    vec![("failed", 2), ("indexed", 4), ("pending", 1)]
  );
  Ok(())
}

/// Two grouping keys make one row per *combination* that occurs — four, not
/// the six the two columns' cardinalities would allow.
#[test]
fn grouping_by_two_columns_returns_one_row_per_combination() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<PairCount> = SelectBuilder::new("source_files")
    .columns_raw(&["source_id", "status"])
    .column_scalar(Scalar::count_star(), "n")
    .group_by(&SOURCE_ID)
    .group_by(&STATUS)
    .order_by(SOURCE_ID.asc())
    .order_by(STATUS.asc())
    .fetch_all(&conn)?;

  assert_eq!(
    rows,
    vec![
      pair("s1", "failed", 2),
      pair("s1", "indexed", 3),
      pair("s1", "pending", 1),
      pair("s2", "indexed", 1),
    ]
  );
  Ok(())
}

/// AC-4: `ORDER BY` names the projection's alias rather than repeating
/// `COUNT(*)`. The three counts are 4 / 2 / 1, so the order is unambiguous.
#[test]
fn ordering_by_an_aggregate_alias_ranks_the_groups() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<KeyCount> = status_counts()
    .order_by(OrderBy::alias_desc("n"))
    .fetch_all(&conn)?;

  assert_eq!(
    pairs(&rows),
    vec![("indexed", 4), ("failed", 2), ("pending", 1)]
  );
  Ok(())
}

#[test]
fn the_ascending_alias_order_reverses_it() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<KeyCount> = status_counts()
    .order_by(OrderBy::alias_asc("n"))
    .fetch_all(&conn)?;

  assert_eq!(
    pairs(&rows),
    vec![("pending", 1), ("failed", 2), ("indexed", 4)]
  );
  Ok(())
}

/// Grouping and pagination compose: the page is a page of *groups*.
#[test]
fn a_grouped_report_paginates_by_group() -> TestResult {
  let conn = db::setup_db()?;

  let rows: Vec<KeyCount> = status_counts()
    .order_by(OrderBy::alias_desc("n"))
    .limit(2)
    .offset(1)
    .fetch_all(&conn)?;

  assert_eq!(pairs(&rows), vec![("failed", 2), ("pending", 1)]);
  Ok(())
}
