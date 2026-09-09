//! Embedded baseline / status against a live Postgres: baselining an adopted
//! schema from a compile-time list, unknown-name rejection, and the first
//! `run_migrate_embedded` after adoption.
//!
//! Needs a live server on `TEST_DB_PORT` (5434 in CI). Does not start Docker.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;

use toolu_orm_cli::migrate::{
  mark_applied_embedded, mark_applied_through_embedded, run_migrate_embedded, MigrateError,
};
use toolu_orm_cli::status::get_status_embedded;
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;
use toolu_orm_core::row::PgCountScalar;

use embedded_list::{honest, list, OwnedMigration, CREATE_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const POSTS_SQL: &str = "CREATE TABLE posts (id TEXT PRIMARY KEY);";
const THIRD_SQL: &str = "CREATE TABLE c (id TEXT);";

async fn adopted_schema(
  schema: &str,
) -> Result<(PgDatabase, PgConnection), Box<dyn std::error::Error>> {
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

fn init_list() -> Vec<OwnedMigration> {
  vec![honest("0001_init.sql", CREATE_SQL)]
}

fn three_list() -> Vec<OwnedMigration> {
  vec![
    honest("0001_init.sql", CREATE_SQL),
    honest("0002_posts.sql", POSTS_SQL),
    honest("0003_c.sql", THIRD_SQL),
  ]
}

#[tokio::test]
async fn baseline_then_migrate_skips_the_baselined_entry_on_postgres() -> TestResult {
  let (_db, conn) = adopted_schema("cli_embedded_baseline_pg").await?;
  let owned = three_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Postgres).await?,
    1
  );
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
    "the recorded hash must come from the embedded entry"
  );
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "no statement from the baselined entry may run"
  );

  assert_eq!(
    run_migrate_embedded(&conn, &migrations, Dialect::Postgres).await?,
    2
  );
  assert_eq!(has_table(&conn, "posts").await?, 1);
  assert_eq!(has_table(&conn, "c").await?, 1);
  assert_eq!(has_table(&conn, "audit").await?, 0);

  let status = get_status_embedded(&conn, &migrations, Dialect::Postgres).await?;
  assert_eq!(
    status.applied,
    ["0001_init.sql", "0002_posts.sql", "0003_c.sql"]
  );
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  Ok(())
}

#[tokio::test]
async fn a_name_absent_from_the_list_records_nothing_on_postgres() -> TestResult {
  let (_db, conn) = adopted_schema("cli_embedded_baseline_pg_unknown").await?;
  let owned = init_list();
  let migrations = list(&owned);

  let Err(err) = mark_applied_embedded(
    &conn,
    &migrations,
    &["0001_init.sql", "0009_ghost.sql"],
    Dialect::Postgres,
  )
  .await
  else {
    return Err("mark_applied_embedded accepted a name absent from the list".into());
  };
  assert!(matches!(err, MigrateError::NotInJournal(_)), "got {err:?}");
  assert!(err.to_string().contains("0009_ghost.sql"), "{err}");
  assert_eq!(
    has_table(&conn, "_migrations").await?,
    0,
    "a rejected baseline must not even create the bookkeeping table"
  );

  assert_eq!(
    mark_applied_embedded(&conn, &migrations, &["0001_init.sql"], Dialect::Postgres).await?,
    1
  );
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 1);
  Ok(())
}

#[tokio::test]
async fn mark_applied_through_embedded_baselines_the_prefix_on_postgres() -> TestResult {
  let (_db, conn) = adopted_schema("cli_embedded_baseline_pg_through").await?;
  let owned = three_list();
  let migrations = list(&owned);

  assert_eq!(
    mark_applied_through_embedded(&conn, &migrations, "0002_posts.sql", Dialect::Postgres).await?,
    2
  );
  let status = get_status_embedded(&conn, &migrations, Dialect::Postgres).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_posts.sql"]);
  assert_eq!(status.pending, ["0003_c.sql"]);
  assert_eq!(has_table(&conn, "posts").await?, 0, "0002 must not run");

  assert_eq!(
    run_migrate_embedded(&conn, &migrations, Dialect::Postgres).await?,
    1
  );
  assert_eq!(has_table(&conn, "c").await?, 1);
  Ok(())
}
