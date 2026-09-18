//! AC-2 and AC-4 on Postgres.

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

#[tokio::test]
async fn grouping_by_one_column_returns_one_row_per_status() -> TestResult {
  let client = db::setup_db("grouping_reports_a").await?;

  let rows: Vec<KeyCount> = status_counts()
    .order_by(STATUS.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(
    pairs(&rows),
    vec![("failed", 2), ("indexed", 4), ("pending", 1)]
  );
  Ok(())
}

#[tokio::test]
async fn grouping_by_two_columns_returns_one_row_per_combination() -> TestResult {
  let client = db::setup_db("grouping_reports_b").await?;

  let rows: Vec<PairCount> = SelectBuilder::new("source_files")
    .columns_raw(&["source_id", "status"])
    .column_scalar(Scalar::count_star(), "n")
    .group_by(&SOURCE_ID)
    .group_by(&STATUS)
    .order_by(SOURCE_ID.asc())
    .order_by(STATUS.asc())
    .fetch_all(&client)
    .await?;

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

#[tokio::test]
async fn ordering_by_an_aggregate_alias_ranks_the_groups() -> TestResult {
  let client = db::setup_db("grouping_reports_c").await?;

  let rows: Vec<KeyCount> = status_counts()
    .order_by(OrderBy::alias_desc("n"))
    .fetch_all(&client)
    .await?;

  assert_eq!(
    pairs(&rows),
    vec![("indexed", 4), ("failed", 2), ("pending", 1)]
  );
  Ok(())
}
