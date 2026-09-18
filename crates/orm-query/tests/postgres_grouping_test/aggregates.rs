//! AC-3 on Postgres, for the aggregates whose return type is `bigint`.
//!
//! `SUM` and `AVG` over a `bigint` are `numeric` here, which the row decoders
//! do not map to a Rust scalar; both SQLite suites cover them.

use toolu_orm_core::expr::Scalar;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, Totals};
use crate::seed::{PATH, SIZE_BYTES, SOURCE_ID};
use crate::support::TestResult;

#[tokio::test]
async fn the_bigint_aggregates_are_computed_per_group() -> TestResult {
  let client = db::setup_db("grouping_aggregates_a").await?;

  let rows: Vec<Totals> = SelectBuilder::new("source_files")
    .columns_raw(&["source_id"])
    .column_scalar(Scalar::count_star(), "rows")
    .column_scalar(Scalar::count_distinct(Scalar::col(&PATH)), "paths")
    .column_scalar(Scalar::max(Scalar::col(&SIZE_BYTES)), "largest")
    .column_scalar(Scalar::min(Scalar::col(&SIZE_BYTES)), "smallest")
    .group_by(&SOURCE_ID)
    .order_by(SOURCE_ID.asc())
    .fetch_all(&client)
    .await?;

  assert_eq!(
    rows,
    vec![
      Totals {
        source_id: "s1".to_owned(),
        rows: 6,
        paths: 3,
        largest: 60,
        smallest: 5,
      },
      Totals {
        source_id: "s2".to_owned(),
        rows: 1,
        paths: 1,
        largest: 50,
        smallest: 50,
      },
    ]
  );
  Ok(())
}
