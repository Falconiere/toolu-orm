//! Applying the generated migrations on a driver without `sqlite-vec`: the
//! named missing module, an FTS5-only migration that still applies, and the
//! driver messages that do and do not map to [`MigrateError::MissingExtension`].

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::{run_migrate, MigrateError};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::vec0::DistanceMetric;

use crate::support::{connect, fts5_registry, migrations_dir, scalar, vec0_registry, TestResult};

#[tokio::test]
async fn migrating_vec0_without_the_extension_is_a_named_missing_module() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(
    &vec0_registry(1024, DistanceMetric::Cosine),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;

  let error = run_migrate(&conn, &dir, Dialect::Sqlite)
    .await
    .expect_err("vec0 DDL must fail without sqlite-vec");
  assert!(
    matches!(
      &error,
      MigrateError::MissingExtension { module, file }
        if module == "vec0" && file == "0001_init.sql"
    ),
    "expected MissingExtension for vec0, got {error}"
  );

  // ensure_migrations_table runs outside the migration transaction, so the
  // table exists; the failed migration's row must not.
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM _migrations WHERE name LIKE '%init%'"
    )
    .await?,
    0
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memories' AND type = 'table'"
    )
    .await?,
    0
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memory_vec'"
    )
    .await?,
    0
  );
  Ok(())
}

#[tokio::test]
async fn an_fts5_only_migration_still_applies_on_the_same_driver() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(&fts5_registry(), &dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memory_fts' AND type = 'table'"
    )
    .await?,
    1
  );
  Ok(())
}

#[test]
fn a_driver_message_without_a_module_name_stays_database() {
  let bare = MigrateError::from_statement("0001_init.sql", "no such module: ");
  assert!(
    matches!(
      &bare,
      MigrateError::Database(message)
        if message.contains("0001_init.sql") && message.contains("no such module:")
    ),
    "expected Database, got {bare}"
  );
  let no_marker = MigrateError::from_statement("0001_init.sql", "syntax error near vec0");
  assert!(
    matches!(&no_marker, MigrateError::Database(_)),
    "expected Database, got {no_marker}"
  );
  let wrapped =
    MigrateError::from_statement("0001_init.sql", "SQLite failure: `no such module: vec0`");
  assert!(
    matches!(
      &wrapped,
      MigrateError::MissingExtension { module, .. } if module == "vec0"
    ),
    "expected MissingExtension, got {wrapped}"
  );
  let bare_rusqlite = MigrateError::from_statement("0001_init.sql", "no such module: vec0");
  assert!(
    matches!(
      &bare_rusqlite,
      MigrateError::MissingExtension { module, .. } if module == "vec0"
    ),
    "expected MissingExtension, got {bare_rusqlite}"
  );
}
