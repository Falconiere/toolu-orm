//! The issue's 16,381-path co-change lookup, executed on libsql.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{SharedBind, SharedBindList};

use crate::db::{setup_db, WeightRow};
use crate::seed::{shared_co_change, working_set, CANDIDATE, EXPECTED_WEIGHT, WORKING_SET};

type Outcome = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn the_issue_query_binds_16_385_parameters_and_returns_the_right_weight() -> Outcome {
  let conn = setup_db().await?;
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let query = shared_co_change(&candidate, &files);
  let (_, params) = query.to_sql_for(Dialect::Sqlite);
  assert_eq!(params.len(), WORKING_SET + 4);
  assert_eq!(params.len(), 16_385);

  let rows: Vec<WeightRow> = query.fetch_all(&conn).await?;

  assert_eq!(
    rows,
    vec![WeightRow {
      weight: EXPECTED_WEIGHT
    }]
  );
  Ok(())
}

#[tokio::test]
async fn two_handles_over_equal_paths_bind_twice_and_still_agree() -> Outcome {
  let conn = setup_db().await?;
  let candidate = SharedBind::new(CANDIDATE);
  let one = SharedBindList::new(working_set());
  let other = SharedBindList::new(working_set());

  let shared: Vec<WeightRow> = shared_co_change(&candidate, &one).fetch_all(&conn).await?;
  let independent = crate::seed::mixed_co_change(&candidate, &one, &other);
  let (_, params) = independent.to_sql_for(Dialect::Sqlite);
  let mixed: Vec<WeightRow> = independent.fetch_all(&conn).await?;

  // Distinct handles stay independent — the statement is 16,381 parameters
  // larger — yet the two forms report the same weight.
  assert_eq!(params.len(), 16_385 + WORKING_SET);
  assert_eq!(shared, mixed);
  assert_eq!(shared, vec![WeightRow { weight: 18 }]);
  Ok(())
}
