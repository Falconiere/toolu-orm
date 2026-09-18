//! AC-7 and AC-8: what `count()` and `exists()` mean once the query is
//! grouped or deduplicated.
//!
//! The settled contract is **the number of rows the unpaginated query
//! returns**. That already described the ungrouped case; with `GROUP BY` the
//! returned rows are groups, so the count is the group count.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, KeyCount};
use crate::seed::SOURCE_ID;
use crate::support::{distinct_paths, status_counts, TestResult};

/// The money assertion. Three traps are live here at once:
///   - 7, the raw row count, is what appending `GROUP BY` to a bare
///     `SELECT COUNT(*)` would *not* return;
///   - 4, the size of the largest group, is what reading the first row of
///     `SELECT COUNT(*) … GROUP BY status` would return;
///   - 3, the number of groups, is the number of rows `fetch_all` returns.
#[tokio::test]
async fn a_grouped_count_reports_the_number_of_groups() -> TestResult {
  let conn = db::setup_db().await?;

  let groups = status_counts().count(&conn).await?;
  let returned: Vec<KeyCount> = status_counts().fetch_all(&conn).await?;

  assert_eq!(groups, 3);
  assert_eq!(groups, i64::try_from(returned.len())?);
  assert_ne!(groups, 7, "not the raw row count");
  assert_ne!(groups, 4, "not the size of the first group");
  Ok(())
}

/// `HAVING` removes groups, so it must change the count too.
#[tokio::test]
async fn having_narrows_the_grouped_count() -> TestResult {
  let conn = db::setup_db().await?;

  let builder = || status_counts().having(Scalar::count_star().gt(Scalar::bind(1i64)));
  let counted = builder().count(&conn).await?;
  let returned: Vec<KeyCount> = builder().fetch_all(&conn).await?;

  assert_eq!(counted, 2);
  assert_eq!(counted, i64::try_from(returned.len())?);
  Ok(())
}

/// A `HAVING` no group satisfies counts zero rather than failing.
#[tokio::test]
async fn a_count_over_no_surviving_group_is_zero() -> TestResult {
  let conn = db::setup_db().await?;

  let counted = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(100i64)))
    .count(&conn)
    .await?;

  assert_eq!(counted, 0);
  Ok(())
}

/// The same argument for `DISTINCT`: `s1` has 3 distinct paths over 6 rows, so
/// counting a distinct listing must report 3.
#[tokio::test]
async fn a_distinct_count_reports_distinct_rows_not_underlying_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let counted = distinct_paths().count(&conn).await?;

  assert_eq!(counted, 3);
  assert_ne!(counted, 6, "not the underlying row count");
  Ok(())
}

/// Pagination never affects a count, before or after this change.
#[tokio::test]
async fn pagination_does_not_change_the_count() -> TestResult {
  let conn = db::setup_db().await?;

  let whole = distinct_paths().count(&conn).await?;
  let paged = distinct_paths().limit(1).offset(1).count(&conn).await?;

  assert_eq!(whole, paged);
  assert_eq!(whole, 3);
  Ok(())
}

/// The regression guard: an ungrouped, non-distinct count is the plain row
/// count it always was.
#[tokio::test]
async fn an_ungrouped_count_still_counts_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let all = SelectBuilder::new("source_files").count(&conn).await?;
  let filtered = SelectBuilder::new("source_files")
    .filter(SOURCE_ID.eq("s1"))
    .count(&conn)
    .await?;

  assert_eq!(all, 7);
  assert_eq!(filtered, 6);
  Ok(())
}

/// AC-8: `exists()` follows `HAVING`, because a `HAVING` can eliminate every
/// group.
#[tokio::test]
async fn exists_follows_the_having_clause() -> TestResult {
  let conn = db::setup_db().await?;

  let some = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .exists(&conn)
    .await?;
  let none = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(100i64)))
    .exists(&conn)
    .await?;

  assert!(some);
  assert!(!none);
  Ok(())
}

/// A grouped `exists()` over a filter that matches nothing is false.
#[tokio::test]
async fn exists_is_false_when_no_group_survives_the_filter() -> TestResult {
  let conn = db::setup_db().await?;

  let found = status_counts()
    .filter(SOURCE_ID.eq("nope"))
    .exists(&conn)
    .await?;

  assert!(!found);
  Ok(())
}
