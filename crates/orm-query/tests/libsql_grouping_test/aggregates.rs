//! AC-3: every aggregate projection, computed by the engine per group.

use toolu_orm_core::expr::Scalar;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{self, IntegerMean, Totals};
use crate::seed::{PATH, SIZE_BYTES, SOURCE_ID};
use crate::support::TestResult;

/// All six aggregates in one select list, grouped by source.
fn totals() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_raw(&["source_id"])
    .column_scalar(Scalar::count_star(), "rows")
    .column_scalar(Scalar::count_distinct(Scalar::col(&PATH)), "paths")
    .column_scalar(Scalar::sum(Scalar::col(&SIZE_BYTES)), "total")
    .column_scalar(Scalar::max(Scalar::col(&SIZE_BYTES)), "largest")
    .column_scalar(Scalar::min(Scalar::col(&SIZE_BYTES)), "smallest")
    .column_scalar(Scalar::avg(Scalar::col(&SIZE_BYTES)), "mean")
    .group_by(&SOURCE_ID)
    .order_by(SOURCE_ID.asc())
}

/// `s1`: 6 rows over 3 distinct paths, sizes 10+20+30+40+60+5 = 165, so the
/// mean is 27.5 — a value integer division could not produce.
///
/// `s2`: a single-row group, where every aggregate collapses to that row.
#[tokio::test]
async fn every_aggregate_is_computed_per_group() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<Totals> = totals().fetch_all(&conn).await?;

  assert_eq!(
    rows,
    vec![
      Totals {
        source_id: "s1".to_owned(),
        rows: 6,
        paths: 3,
        total: 165,
        largest: 60,
        smallest: 5,
        mean: 27.5,
      },
      Totals {
        source_id: "s2".to_owned(),
        rows: 1,
        paths: 1,
        total: 50,
        largest: 50,
        smallest: 50,
        mean: 50.0,
      },
    ]
  );
  Ok(())
}

/// `COUNT(DISTINCT path)` is the shape `Scalar::func` cannot express, and it
/// differs from `COUNT(*)` exactly where duplicates exist: 3 against 6.
#[tokio::test]
async fn count_distinct_differs_from_count_star_where_duplicates_exist() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<Totals> = totals().fetch_all(&conn).await?;
  let first = rows.first().ok_or("expected the s1 group")?;

  assert_eq!(first.rows, 6);
  assert_eq!(first.paths, 3);
  Ok(())
}

/// An aggregate is a `Scalar`, so it composes with arithmetic. SQLite divides
/// two integers as integers: 165 / 6 is 27, not 27.5 — which is exactly why
/// `AVG` exists and is asserted separately above.
#[tokio::test]
async fn aggregates_compose_with_arithmetic_in_a_projection() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<IntegerMean> = SelectBuilder::new("source_files")
    .columns_raw(&["source_id"])
    .column_scalar(Scalar::sum(Scalar::col(&SIZE_BYTES)), "total")
    .column_scalar(
      Scalar::sum(Scalar::col(&SIZE_BYTES)) / Scalar::count_star(),
      "mean",
    )
    .group_by(&SOURCE_ID)
    .order_by(SOURCE_ID.asc())
    .fetch_all(&conn)
    .await?;

  let first = rows.first().ok_or("expected the s1 group")?;
  assert_eq!(first.total, 165);
  assert_eq!(first.mean, 27);
  Ok(())
}
