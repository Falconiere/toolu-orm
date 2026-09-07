//! Generate → migrate → evolve → generate → migrate against a live Postgres,
//! asserting the applied schema through `information_schema` and `pg_indexes`,
//! plus mid-file failure rollback.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Runs on the five-crate postgres lane only.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "fixtures/loop_registry.rs"]
pub mod loop_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::{run_migrate, MigrateError};
use toolu_orm_cli::status::get_status;
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{compute_hash, Journal};
use toolu_orm_core::row::PgCountScalar;
use toolu_orm_core::value::Value;

use loop_registry::{migrations_dir, registry_v1, registry_v2};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn pg_schema_conn(
  schema: &str,
) -> Result<(PgDatabase, PgConnection), Box<dyn std::error::Error>> {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; SET search_path TO {schema}"
    ))
    .await?;
  Ok((db, conn))
}

async fn count(conn: &PgConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows = conn.query_map::<PgCountScalar>(sql, vec![]).await?;
  Ok(rows.first().ok_or("count query returned no row")?.value)
}

const COLUMNS_IN_USERS: &str = "SELECT count(*) FROM information_schema.columns \
   WHERE table_schema = current_schema() AND table_name = 'users'";

#[tokio::test]
async fn loop_v1_then_v2_applies_every_change_on_postgres() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = pg_schema_conn("cli_loop_pg").await?;

  run_generate(&registry_v1(), &dir, "init", Dialect::Postgres)?;
  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 1);
  assert_eq!(count(&conn, COLUMNS_IN_USERS).await?, 4);

  run_generate(&registry_v2(), &dir, "evolve", Dialect::Postgres)?;
  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 1);
  assert_eq!(
    count(
      &conn,
      &format!("{COLUMNS_IN_USERS} AND column_name = 'bio'")
    )
    .await?,
    1
  );
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM information_schema.columns \
       WHERE table_schema = current_schema() AND table_name = 'posts' AND column_name = 'status'"
    )
    .await?,
    1
  );
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM pg_indexes \
       WHERE schemaname = current_schema() AND indexname = 'idx_users_email'"
    )
    .await?,
    1
  );
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM information_schema.table_constraints \
       WHERE table_schema = current_schema() AND table_name = 'posts' \
       AND constraint_type = 'FOREIGN KEY'"
    )
    .await?,
    1
  );
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM information_schema.check_constraints \
       WHERE constraint_schema = current_schema() AND check_clause LIKE '%banned%'"
    )
    .await?,
    1
  );

  let status = get_status(&conn, &dir, Dialect::Postgres).await?;
  assert_eq!(status.applied, ["0001_init.sql", "0002_evolve.sql"]);
  assert!(status.pending.is_empty(), "pending: {:?}", status.pending);
  assert_eq!(
    run_generate(&registry_v2(), &dir, "noop", Dialect::Postgres)?,
    None
  );

  let user = |id: &str, email: &str| {
    vec![
      Value::Text(id.into()),
      Value::Text("Ann".into()),
      Value::Text(email.into()),
    ]
  };
  let post = |id: &str, status: &str| {
    vec![
      Value::Text(id.into()),
      Value::Text("u1".into()),
      Value::Text("Hello".into()),
      Value::Text(status.into()),
    ]
  };
  let insert_user = "INSERT INTO users (id, name, email) VALUES ($1, $2, $3)";
  let insert_post = "INSERT INTO posts (id, author_id, title, status) VALUES ($1, $2, $3, $4)";
  assert_eq!(
    conn
      .execute_sql(insert_user, user("u1", "ann@x.io"))
      .await?,
    1
  );
  assert_eq!(
    conn.execute_sql(insert_post, post("p1", "active")).await?,
    1
  );
  assert!(
    conn
      .execute_sql(insert_post, post("p2", "bogus"))
      .await
      .is_err(),
    "CHECK on posts.status was not applied"
  );
  assert!(
    conn
      .execute_sql(insert_user, user("u2", "ann@x.io"))
      .await
      .is_err(),
    "unique index on email was not applied"
  );
  Ok(())
}

#[tokio::test]
async fn mid_file_failure_rolls_back_on_postgres() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = pg_schema_conn("cli_loop_pg_fail").await?;

  let sql = "CREATE TABLE a (id TEXT);\n--> statement-breakpoint\nINSERT INTO nope VALUES (1);";
  std::fs::write(format!("{dir}/0001_bad.sql"), sql)?;
  let mut journal = Journal::empty();
  journal.add_entry("0001_bad.sql", &compute_hash(sql));
  journal.write_to_path(&format!("{dir}/_journal.json"))?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Postgres).await else {
    return Err("migration with a failing statement unexpectedly succeeded".into());
  };
  assert!(matches!(err, MigrateError::Database(_)), "got {err:?}");
  assert_eq!(count(&conn, "SELECT count(*) FROM _migrations").await?, 0);
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM information_schema.tables \
       WHERE table_schema = current_schema() AND table_name = 'a'"
    )
    .await?,
    0
  );
  Ok(())
}

#[tokio::test]
async fn legacy_mode_applies_plain_sql_files_on_postgres() -> TestResult {
  // No `_journal.json`: the runner scans the directory and applies each file
  // as one batch, recording it with an empty hash.
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = pg_schema_conn("cli_loop_pg_legacy").await?;
  std::fs::write(
    format!("{dir}/0001_init.sql"),
    "CREATE TABLE a (id TEXT PRIMARY KEY);\nCREATE TABLE b (a_id TEXT REFERENCES a(id));",
  )?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 1);
  assert_eq!(run_migrate(&conn, &dir, Dialect::Postgres).await?, 0);
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM information_schema.tables \
       WHERE table_schema = current_schema() AND table_name IN ('a', 'b')"
    )
    .await?,
    2
  );
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM _migrations WHERE name = '0001_init.sql' AND hash = ''"
    )
    .await?,
    1
  );
  let status = get_status(&conn, &dir, Dialect::Postgres).await?;
  assert_eq!(status.applied, ["0001_init.sql"]);
  assert!(status.pending.is_empty());
  Ok(())
}
