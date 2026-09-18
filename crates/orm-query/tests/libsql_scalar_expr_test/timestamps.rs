//! AC-2: `datetime(...)` compares mixed-precision RFC3339 stamps correctly,
//! where a plain string comparison does not.

use toolu_orm_core::expr::Scalar;

use crate::db::{self, ids, Memory};
use crate::seed::{CREATED_AT, CUTOFF, ID};
use crate::support::{all_memories, created_datetime, TestResult};

#[tokio::test]
async fn datetime_normalizes_mixed_precision_timestamps() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<Memory> = all_memories()
    .filter(created_datetime()?.gte(Scalar::func("datetime", vec![Scalar::bind(CUTOFF)])?))
    .order_by(ID.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(&rows), vec!["m1", "m2"]);
  Ok(())
}

#[tokio::test]
async fn a_plain_string_comparison_drops_the_sub_second_row() -> TestResult {
  let conn = db::setup_db().await?;

  let rows: Vec<Memory> = all_memories()
    .filter(Scalar::col(&CREATED_AT).gte(Scalar::bind(CUTOFF)))
    .order_by(ID.asc())
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(&rows), vec!["m1"]);
  Ok(())
}
