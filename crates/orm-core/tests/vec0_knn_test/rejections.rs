//! Everything the vec0 KNN surface refuses: Postgres, non-positive `k`.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, Vec0Ops};
use toolu_orm_core::value::Value;
use toolu_orm_core::vec0;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("memory_vec", "embedding");

fn unsupported_dialect(
  error: DbCoreError,
  expected_feature: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let message = error.to_string();
  let DbCoreError::Vec0UnsupportedDialect { feature, dialect } = error else {
    return Err(format!("expected Vec0UnsupportedDialect, got: {error:?}").into());
  };
  assert_eq!(feature, expected_feature);
  assert_eq!(dialect, "postgres");
  Ok(message)
}

fn invalid_argument(
  error: DbCoreError,
  expected_feature: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let DbCoreError::Vec0InvalidArgument { feature, reason } = error else {
    return Err(format!("expected Vec0InvalidArgument, got: {error:?}").into());
  };
  assert_eq!(feature, expected_feature);
  Ok(reason)
}

#[test]
fn postgres_is_refused_by_match_k_and_distance() -> TestResult {
  let match_err = EMBEDDING
    .matches_for(Dialect::Postgres, Value::vector(&[1.0f32]))
    .err()
    .ok_or("MATCH must refuse Dialect::Postgres")?;
  let message = unsupported_dialect(match_err, "MATCH")?;
  assert!(
    message.contains("postgres") && message.contains("pgvector"),
    "the message must name postgres and point at pgvector: {message}"
  );

  let k_err = vec0::k_eq_for(Dialect::Postgres, 10)
    .err()
    .ok_or("k must refuse Dialect::Postgres")?;
  unsupported_dialect(k_err, "k")?;

  let distance_err = vec0::distance_for(Dialect::Postgres)
    .err()
    .ok_or("distance must refuse Dialect::Postgres")?;
  unsupported_dialect(distance_err, "distance")?;
  Ok(())
}

#[test]
fn non_positive_k_is_refused() -> TestResult {
  let zero = vec0::k_eq_for(Dialect::Sqlite, 0)
    .err()
    .ok_or("k = 0 must be refused")?;
  let reason = invalid_argument(zero, "k")?;
  assert!(
    reason.contains('0'),
    "reason should name the value: {reason}"
  );

  let negative = vec0::k_eq_for(Dialect::Sqlite, -1)
    .err()
    .ok_or("k = -1 must be refused")?;
  let reason = invalid_argument(negative, "k")?;
  assert!(
    reason.contains("-1"),
    "reason should name the value: {reason}"
  );
  Ok(())
}
