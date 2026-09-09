//! Everything the pgvector surface refuses: SQLite dialect and non-finite floats.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::pgvector::PgVectorOps;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::vec0;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("items", "embedding");

fn unsupported_dialect(
  error: DbCoreError,
  expected_feature: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let message = error.to_string();
  let DbCoreError::PgVectorUnsupportedDialect { feature, dialect } = error else {
    return Err(format!("expected PgVectorUnsupportedDialect, got: {error:?}").into());
  };
  assert_eq!(feature, expected_feature);
  assert_eq!(dialect, "sqlite");
  Ok(message)
}

#[test]
fn sqlite_is_refused_by_every_constructor() -> TestResult {
  let l2 = EMBEDDING
    .l2_distance_for(Dialect::Sqlite, &[1.0])
    .err()
    .ok_or("l2 must refuse Sqlite")?;
  let message = unsupported_dialect(l2, "l2_distance")?;
  assert!(message.contains("sqlite-vec"));
  assert!(message.contains("pgvector"));

  unsupported_dialect(
    EMBEDDING
      .cosine_distance_for(Dialect::Sqlite, &[1.0])
      .err()
      .ok_or("cosine")?,
    "cosine_distance",
  )?;
  unsupported_dialect(
    EMBEDDING
      .neg_inner_product_for(Dialect::Sqlite, &[1.0])
      .err()
      .ok_or("nip")?,
    "neg_inner_product",
  )?;
  Ok(())
}

#[test]
fn a_non_finite_embedding_element_is_refused() -> TestResult {
  let nan = EMBEDDING
    .l2_distance_for(Dialect::Postgres, &[f32::NAN, 0.0])
    .err()
    .ok_or("NaN")?;
  let DbCoreError::PgVectorInvalidArgument { feature, reason } = nan else {
    return Err(format!("expected PgVectorInvalidArgument, got: {nan:?}").into());
  };
  assert_eq!(feature, "l2_distance");
  assert!(reason.contains("finite"));

  let inf = EMBEDDING
    .l2_distance_for(Dialect::Postgres, &[f32::INFINITY])
    .err()
    .ok_or("Inf")?;
  assert!(matches!(inf, DbCoreError::PgVectorInvalidArgument { .. }));
  Ok(())
}

#[test]
fn vec0_still_refuses_postgres_and_pgvector_refuses_sqlite() -> TestResult {
  let vec0_err = vec0::distance_for(Dialect::Postgres)
    .err()
    .ok_or("vec0 distance must still refuse Postgres")?;
  assert!(matches!(
    vec0_err,
    DbCoreError::Vec0UnsupportedDialect {
      ref feature,
      dialect
    } if feature == "distance" && dialect == "postgres"
  ));

  let pg = EMBEDDING
    .l2_distance_for(Dialect::Sqlite, &[1.0])
    .err()
    .ok_or("pgvector must refuse Sqlite")?;
  assert!(matches!(
    pg,
    DbCoreError::PgVectorUnsupportedDialect {
      ref feature,
      dialect
    } if feature == "l2_distance" && dialect == "sqlite"
  ));
  Ok(())
}
