//! `run_migrate_embedded` against a live Postgres: applying a compile-time
//! list, re-running it, a tampered body, and a failing statement, asserted
//! through `information_schema` and `_migrations` on an owned schema.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Runs on the postgres lane only.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{run_migrate_embedded, MigrateError};
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;
use toolu_orm_core::row::PgCountScalar;

use embedded_list::{honest, list, tampered, OwnedMigration, CREATE_SQL, FAILING_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

/// A fresh schema of this test's own, so the suite can run in parallel.
async fn owned_schema(
  schema: &str,
) -> Result<(PgDatabase, PgConnection), Box<dyn std::error::Error>> {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
       SET search_path TO {schema}"
    ))
    .await?;
  Ok((db, conn))
}

async fn migrate(conn: &PgConnection, owned: &[OwnedMigration]) -> Result<u32, MigrateError> {
  run_migrate_embedded(conn, &list(owned), Dialect::Postgres).await
}

async fn count(conn: &PgConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows = conn.query_map::<PgCountScalar>(sql, vec![]).await?;
  Ok(rows.first().ok_or("count query returned no row")?.value)
}

/// 1 when the table exists in the test's schema, 0 when it does not.
async fn has_table(conn: &PgConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  count(
    conn,
    &format!(
      "SELECT count(*) FROM information_schema.tables \
       WHERE table_schema = current_schema() AND table_name = '{name}'"
    ),
  )
  .await
}

fn two_migrations() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
  ]
}

#[tokio::test]
async fn an_embedded_list_applies_and_records_every_migration_on_postgres() -> TestResult {
  let (_db, conn) = owned_schema("cli_embedded_pg").await?;
  let mut migrations = two_migrations();

  assert_eq!(migrate(&conn, &migrations).await?, 2);
  assert_eq!(has_table(&conn, "users").await?, 1);
  assert_eq!(
    has_table(&conn, "audit").await?,
    1,
    "the statement after the breakpoint must run too"
  );
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(
    count(
      &conn,
      &format!(
        "SELECT count(*) FROM _migrations WHERE name = '0001_init.sql' AND hash = '{}'",
        compute_hash(CREATE_SQL)
      )
    )
    .await?,
    1,
    "the declared hash must be what is recorded"
  );

  assert_eq!(migrate(&conn, &migrations).await?, 0);
  migrations.push(honest("0003_c.sql", THIRD_SQL));
  assert_eq!(migrate(&conn, &migrations).await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 3);
  Ok(())
}

#[tokio::test]
async fn an_edited_migration_fails_the_hash_check_on_postgres() -> TestResult {
  let (_db, conn) = owned_schema("cli_embedded_pg_tamper").await?;
  let migrations = vec![
    honest("0001_init.sql", CREATE_SQL),
    tampered(
      "0002_posts.sql",
      POSTS_SQL,
      "CREATE TABLE posts (id TEXT PRIMARY KEY, tampered TEXT);",
    ),
  ];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a tampered embedded migration was applied".into());
  };
  assert!(
    matches!(err, MigrateError::HashMismatch { ref file, .. } if file == "0002_posts.sql"),
    "got {err:?}"
  );
  assert_eq!(has_table(&conn, "posts").await?, 0);
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM _migrations WHERE name = '0001_init.sql'"
    )
    .await?,
    1,
    "the valid migration before it stays applied"
  );
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_failing_statement_rolls_back_only_its_own_migration_on_postgres() -> TestResult {
  let (_db, conn) = owned_schema("cli_embedded_pg_fail").await?;
  let migrations = vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_bad.sql", FAILING_SQL),
  ];

  let Err(err) = migrate(&conn, &migrations).await else {
    return Err("a failing statement did not stop the run".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert!(err.to_string().contains("0002_bad.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "half").await?,
    0,
    "the first statement's table survived the rollback"
  );
  assert_eq!(has_table(&conn, "users").await?, 1);
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}
