//! AC-5 and AC-6 on Postgres: a `HAVING` bind arrives as `$N`, numbered after
//! the `WHERE` bind.
//!
//! The second conjunct compares `MAX` rather than `SUM`: `max(bigint)` is
//! `bigint`, which a bound `Value::Integer` matches, while `sum(bigint)` is
//! `numeric`.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, pairs, KeyCount, MaybeLargest};
use crate::seed::{SIZE_BYTES, SOURCE_ID, STATUS};
use crate::support::{status_counts, TestResult};

#[tokio::test]
async fn having_filters_groups_by_their_aggregate() -> TestResult {
  let client = db::setup_db("grouping_having_a").await?;

  let rows: Vec<KeyCount> = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(STATUS.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(pairs(&rows), vec![("failed", 2), ("indexed", 4)]);
  Ok(())
}

/// Mis-numbering would compare each bind against the other's value.
#[tokio::test]
async fn a_where_bind_and_a_having_bind_keep_their_own_values() -> TestResult {
  let client = db::setup_db("grouping_having_b").await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .filter(SOURCE_ID.eq("s1"))
    .group_by(&STATUS)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .order_by(STATUS.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(pairs(&rows), vec![("failed", 2), ("indexed", 3)]);
  Ok(())
}

/// Within `s1`: failed is 2 rows with a largest of 40, indexed 3 rows with a
/// largest of 60. `COUNT(*) > 1` keeps both; `MAX < 50` then drops indexed.
#[tokio::test]
async fn several_having_conjuncts_all_apply() -> TestResult {
  let client = db::setup_db("grouping_having_c").await?;

  let rows: Vec<KeyCount> = SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .filter(SOURCE_ID.eq("s1"))
    .group_by(&STATUS)
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .having(Scalar::max(Scalar::col(&SIZE_BYTES)).lt(Scalar::bind(50i64)))
    .fetch_all(&client)
    .await?;

  assert_eq!(pairs(&rows), vec![("failed", 2)]);
  Ok(())
}

#[tokio::test]
async fn a_having_that_excludes_every_group_returns_no_rows() -> TestResult {
  let client = db::setup_db("grouping_having_d").await?;

  let rows: Vec<KeyCount> = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(100i64)))
    .fetch_all(&client)
    .await?;

  assert!(rows.is_empty());
  Ok(())
}

/// The sharp edge of AC-6: an aggregate with no `GROUP BY` over zero matching
/// rows still returns exactly one row — `MAX` NULL, `COUNT(*)` zero.
#[tokio::test]
async fn an_ungrouped_aggregate_over_no_rows_returns_one_null_row() -> TestResult {
  let client = db::setup_db("grouping_having_e").await?;

  let rows: Vec<MaybeLargest> = SelectBuilder::new("source_files")
    .column_scalar(Scalar::max(Scalar::col(&SIZE_BYTES)), "largest")
    .column_scalar(Scalar::count_star(), "rows")
    .filter(SOURCE_ID.eq("nope"))
    .fetch_all(&client)
    .await?;

  assert_eq!(
    rows,
    vec![MaybeLargest {
      largest: None,
      rows: 0
    }]
  );
  Ok(())
}
