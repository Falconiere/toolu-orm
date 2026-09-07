//! `mark_applied` / `mark_applied_through` against a live Postgres: baselining
//! an already-migrated schema, the first real migrate after adoption, and the
//! unknown-name rejection, asserted through `information_schema` and
//! `_migrations`.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Runs on the postgres lane only.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;

use toolu_orm_cli::migrate::{mark_applied, mark_applied_through, run_migrate, MigrateError};
use toolu_orm_cli::status::get_status;
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;
use toolu_orm_core::row::PgCountScalar;

use baseline_dir::{migrations_dir, write_migrations, INIT_SQL, POSTS_SQL, THIRD_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An owned schema already carrying `users` (a prior migration system put it
/// there) plus a migrations dir whose `0001_init.sql` would recreate it.
async fn adopted_schema(
  schema: &str,
  dir: &str,
) -> Result<(PgDatabase, PgConnection), Box<dyn std::error::Error>> {
  write_migrations(dir, &[("0001_init.sql", INIT_SQL)])?;

  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; \
       SET search_path TO {schema}; CREATE TABLE users (id TEXT PRIMARY KEY)"
    ))
    .await?;
  Ok((db, conn))
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

#[tokio::test]
async fn baseline_then_migrate_skips_the_baselined_entry_on_postgres() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = adopted_schema("cli_baseline_pg", &dir).await?;

  assert_eq!(
    mark_applied(&conn, &dir, &["0001_init.sql"], Dialect::Postgres).await?,
    1
  );
  assert_eq!(
    count(
      &conn,
      &format!(
        "SELECT count(*) FROM _migrations WHERE name = '0001_init.sql' AND hash = '{}'",
        compute_hash(INIT_SQL)
      )
    )
    .await?,
    1,
    "the recorded hash must come from the journal"
  );
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "no statement from the baselined file may run"
  );

  write_migrations(
    &dir,
    &[("0001_init.sql", INIT_SQL), ("0002_posts.sql", POSTS_SQL)],
  )?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 1);
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(has_table(&conn, "audit").await?, 0);

  let status = get_status(&conn, &dir, Dialect::Postgres).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  Ok(())
}

#[tokio::test]
async fn a_name_absent_from_the_journal_records_nothing_on_postgres() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = adopted_schema("cli_baseline_pg_unknown", &dir).await?;

  let Err(err) = mark_applied(
    &conn,
    &dir,
    &["0001_init.sql", "0009_ghost.sql"],
    Dialect::Postgres,
  )
  .await
  else {
    return Err("mark_applied accepted a name with no journal entry".into());
  };
  assert!(matches!(err, MigrateError::NotInJournal(_)), "got {err:?}");
  assert!(err.to_string().contains("0009_ghost.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "_migrations").await?,
    0,
    "a rejected baseline must not even create the bookkeeping table"
  );

  // The valid name from the rejected call was not recorded: baselining it alone
  // still counts it as new, and leaves exactly one row.
  assert_eq!(
    mark_applied(&conn, &dir, &["0001_init.sql"], Dialect::Postgres).await?,
    1
  );
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn mark_applied_through_baselines_the_prefix_on_postgres() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = adopted_schema("cli_baseline_pg_through", &dir).await?;
  write_migrations(
    &dir,
    &[
      ("0001_init.sql", INIT_SQL),
      ("0002_posts.sql", POSTS_SQL),
      ("0003_c.sql", THIRD_SQL),
    ],
  )?;

  assert_eq!(
    mark_applied_through(&conn, &dir, "0002_posts.sql", Dialect::Postgres).await?,
    2
  );
  let status = get_status(&conn, &dir, Dialect::Postgres).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert_eq!(status.pending, ["0003_c.sql"]);
  assert_eq!(has_table(&conn, "posts").await?, 0, "0002 must not run");

  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  Ok(())
}
