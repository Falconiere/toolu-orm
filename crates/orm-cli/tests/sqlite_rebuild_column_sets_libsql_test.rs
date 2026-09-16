//! One rebuild absorbs every column change the same diff makes to a table.
//!
//! Issue #85: an `AlterColumn` and an `AddColumn` on the same table produced a
//! rebuild that selected the new column out of the old table, and the migration
//! failed. The rebuild now carries the whole target definition and is emitted
//! once per table.

#[path = "fixtures/libsql_probe.rs"]
pub mod libsql_probe;
#[path = "fixtures/rebuild_registry.rs"]
pub mod rebuild_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::LibsqlConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::schema::SchemaRegistry;

use libsql_probe::{column_names, connect, exec, scalar, seed_user_and_post};
use rebuild_registry::{migrations_dir, registry_v1, registry_v3, registry_v4};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// v1 applied and seeded, then `target` applied in one migration.
async fn evolve_from_v1(conn: &LibsqlConnection, dir: &str, target: &SchemaRegistry) -> TestResult {
  exec(conn, "PRAGMA foreign_keys = ON").await?;
  run_generate(&registry_v1(), dir, "init", Dialect::Sqlite)?;
  run_migrate(conn, dir, Dialect::Sqlite).await?;
  seed_user_and_post(conn).await?;
  run_generate(target, dir, "evolve", Dialect::Sqlite)?;
  assert_eq!(run_migrate(conn, dir, Dialect::Sqlite).await?, 1);
  Ok(())
}

/// The rebuilt table exists exactly once and left no staging table behind.
async fn assert_single_users_table(conn: &LibsqlConnection) -> TestResult {
  assert_eq!(
    scalar(
      conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'users'"
    )
    .await?,
    1
  );
  assert_eq!(
    scalar(
      conn,
      "SELECT count(*) FROM sqlite_master WHERE name LIKE '_toolu_new_%'"
    )
    .await?,
    0,
    "a staging table outlived the rebuild"
  );
  Ok(())
}

#[tokio::test]
async fn altering_and_adding_a_column_applies_in_one_rebuild() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;

  evolve_from_v1(&conn, &dir, &registry_v3()).await?;

  assert_eq!(
    column_names(&conn, "users").await?,
    ["id", "name", "email", "bio", "age"]
  );
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM users").await?,
    1,
    "the existing row did not survive the rebuild"
  );
  assert_eq!(
    scalar(&conn, "SELECT age FROM users WHERE id = 'u1'").await?,
    7,
    "the added column did not take its declared default"
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  assert_single_users_table(&conn).await
}

#[tokio::test]
async fn altering_and_dropping_a_column_applies_in_one_rebuild() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;

  evolve_from_v1(&conn, &dir, &registry_v4()).await?;

  assert_eq!(column_names(&conn, "users").await?, ["id", "name", "email"]);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM users").await?, 1);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM posts").await?, 1);
  let still_nullable = exec(
    &conn,
    "INSERT INTO users (id, email) VALUES ('u2', 'bob@x.io')",
  )
  .await;
  assert!(
    still_nullable.is_err(),
    "name is still nullable, so the alter half of the diff was lost"
  );
  assert_single_users_table(&conn).await
}
