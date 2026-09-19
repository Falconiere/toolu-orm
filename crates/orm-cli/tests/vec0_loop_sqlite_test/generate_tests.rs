//! Regenerating a `vec0` schema: identical is a no-op, and a dimension or
//! metric change is refused before any migration file is written.

use toolu_orm_cli::generate::run_generate;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::vec0::DistanceMetric;

use crate::support::{migrations_dir, vec0_registry, TestResult};

#[tokio::test]
async fn regenerating_the_same_vec0_schema_finds_no_change() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;
  assert_eq!(
    run_generate(
      &vec0_registry(1024, DistanceMetric::Cosine),
      &dir,
      "noop",
      Dialect::Sqlite
    )?,
    None
  );
  Ok(())
}

#[tokio::test]
async fn changing_the_dimension_is_refused_without_writing_a_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_generate(
    &vec0_registry(768, DistanceMetric::Cosine),
    &dir,
    "redim",
    Dialect::Sqlite,
  )
  .expect_err("expected the virtual-table change to be refused");
  let message = error.to_string();
  assert!(
    message.contains("memory_vec") && message.contains("columns"),
    "unhelpful error: {message}"
  );
  assert!(
    !std::path::Path::new(&dir).join("0002_redim.sql").exists(),
    "a migration was written for a refused change"
  );
  Ok(())
}

#[tokio::test]
async fn changing_the_distance_metric_is_refused_without_writing_a_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_generate(
    &vec0_registry(1024, DistanceMetric::L2),
    &dir,
    "remetric",
    Dialect::Sqlite,
  )
  .expect_err("expected the virtual-table change to be refused");
  let message = error.to_string();
  assert!(
    message.contains("memory_vec") && message.contains("module arguments"),
    "unhelpful error: {message}"
  );
  assert!(
    !std::path::Path::new(&dir)
      .join("0002_remetric.sql")
      .exists(),
    "a migration was written for a refused change"
  );
  Ok(())
}
