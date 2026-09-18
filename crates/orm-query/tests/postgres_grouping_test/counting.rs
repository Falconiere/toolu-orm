//! AC-7 and AC-8 on Postgres.
//!
//! The lane that matters most for the count wrap: Postgres *requires* an alias
//! on a subquery in `FROM`, so `SELECT COUNT(*) FROM (…) AS "toolu_count"`
//! either parses here or the whole design is wrong.

use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, KeyCount};
use crate::seed::SOURCE_ID;
use crate::support::{distinct_paths, status_counts, TestResult};

#[tokio::test]
async fn a_grouped_count_reports_the_number_of_groups() -> TestResult {
  let client = db::setup_db("grouping_counting_a").await?;

  let groups = status_counts().count(&client).await?;
  let returned: Vec<KeyCount> = status_counts().fetch_all(&client).await?;

  assert_eq!(groups, 3);
  assert_eq!(groups, i64::try_from(returned.len())?);
  assert_ne!(groups, 7, "not the raw row count");
  assert_ne!(groups, 4, "not the size of the first group");
  Ok(())
}

#[tokio::test]
async fn having_narrows_the_grouped_count() -> TestResult {
  let client = db::setup_db("grouping_counting_b").await?;

  let counted = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .count(&client)
    .await?;

  assert_eq!(counted, 2);
  Ok(())
}

#[tokio::test]
async fn a_distinct_count_reports_distinct_rows_not_underlying_rows() -> TestResult {
  let client = db::setup_db("grouping_counting_c").await?;

  let counted = distinct_paths().count(&client).await?;

  assert_eq!(counted, 3);
  assert_ne!(counted, 6, "not the underlying row count");
  Ok(())
}

#[tokio::test]
async fn an_ungrouped_count_still_counts_rows() -> TestResult {
  let client = db::setup_db("grouping_counting_d").await?;

  let all = SelectBuilder::new("source_files").count(&client).await?;
  let filtered = SelectBuilder::new("source_files")
    .filter(SOURCE_ID.eq("s1"))
    .count(&client)
    .await?;

  assert_eq!(all, 7);
  assert_eq!(filtered, 6);
  Ok(())
}

#[tokio::test]
async fn exists_follows_the_having_clause() -> TestResult {
  let client = db::setup_db("grouping_counting_e").await?;

  let some = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(1i64)))
    .exists(&client)
    .await?;
  let none = status_counts()
    .having(Scalar::count_star().gt(Scalar::bind(100i64)))
    .exists(&client)
    .await?;

  assert!(some);
  assert!(!none);
  Ok(())
}
