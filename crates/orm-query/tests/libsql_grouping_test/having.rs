//! AC-5 and AC-6: `HAVING` filters groups with bound parameters, and the
//! shapes that come back empty.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, pairs, KeyCount, MaybeTotal};
use crate::seed::{SIZE_BYTES, SOURCE_ID, STATUS};
use crate::support::{status_counts, TestResult};

/// `HAVING COUNT(*) > ?` keeps the groups larger than one: `indexed` (4) and
/// `failed` (2), never `pending` (1).
#[tokio::test]
async fn having_filters_groups_by_their_aggregate() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(STATUS.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(pairs(&rows), vec![("failed", 2), ("indexed", 4)]);
  Ok(())
}

/// The bound `HAVING` value is a real parameter: changing it changes the rows
/// without changing the SQL.
#[tokio::test]
async fn the_having_bound_is_a_parameter_not_a_literal() -> TestResult {
  let conn = db::setup_db().await?;

  let above_three: Vec<KeyCount> = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(3i64)))
    .fetch_all(&conn)
    .await?;

  assert_eq!(pairs(&above_three), vec![("indexed", 4)]);
  Ok(())
}

/// AC-5's numbering case: a `WHERE` bind and a `HAVING` bind in one statement,
/// where mis-numbering would silently compare against the wrong value.
#[tokio::test]
async fn a_where_bind_and_a_having_bind_keep_their_own_values() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .filter(SOURCE_ID.eq("s1"))
    .group_by(&STATUS)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(STATUS.asc())
    .fetch_all(&conn)
    .await?;

  // Within s1: failed 2, indexed 3, pending 1 — so pending drops out.
  assert_eq!(pairs(&rows), vec![("failed", 2), ("indexed", 3)]);
  Ok(())
}

/// Two conjuncts, `AND`-joined. Within `s1`: failed is 2 rows summing 45,
/// indexed 3 rows summing 90, pending 1 row summing 30. `COUNT(*) > 1` keeps
/// failed and indexed; `SUM < 50` then drops indexed.
#[tokio::test]
async fn several_having_conjuncts_all_apply() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .filter(SOURCE_ID.eq("s1"))
    .group_by(&STATUS)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .having(Scalar::sum(Scalar::col(&SIZE_BYTES)).lt(Scalar::bind(50i64)))
    .fetch_all(&conn)
    .await?;

  assert_eq!(pairs(&rows), vec![("failed", 2)]);
  Ok(())
}

/// AC-6: a `HAVING` no group satisfies yields an empty result, not an error.
#[tokio::test]
async fn a_having_that_excludes_every_group_returns_no_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(100i64)))
    .fetch_all(&conn)
    .await?;

  assert!(rows.is_empty());
  Ok(())
}

/// A `WHERE` matching nothing leaves no groups at all — `GROUP BY` over an
/// empty set produces zero rows.
#[tokio::test]
async fn grouping_an_empty_filter_result_returns_no_rows() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<KeyCount> = status_counts()
    .filter(SOURCE_ID.eq("nope"))
    .fetch_all(&conn)
    .await?;

  assert!(rows.is_empty());
  Ok(())
}

/// The sharp edge of AC-6, and the one that catches people: an aggregate with
/// **no** `GROUP BY` over zero matching rows still returns exactly one row —
/// `SUM` NULL, `COUNT(*)` zero.
#[tokio::test]
async fn an_ungrouped_aggregate_over_no_rows_returns_one_null_row() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<MaybeTotal> = SelectBuilder::new("source_files")
    .column_scalar(Scalar::sum(Scalar::col(&SIZE_BYTES)), "total")
    .column_scalar(Scalar::count_star(), "rows")
    .filter(SOURCE_ID.eq("nope"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(
    rows,
    vec![MaybeTotal {
      total: None,
      rows: 0
    }]
  );
  Ok(())
}
