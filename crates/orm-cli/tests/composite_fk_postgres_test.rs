//! Table-level composite foreign keys on a live Postgres: the issue's schema
//! shape — unique `(id, project_id)` targets, a self-reference and a
//! cascading key — migrates, and the constraints hold.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434`; each test owns a schema and fails hard without the
//! server.

#[path = "fixtures/composite_fk_registry.rs"]
pub mod composite_fk_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::row::PgCountScalar;
use toolu_orm_core::schema::SchemaRegistry;

use composite_fk_registry::{
  migrations_dir, registry_v1, registry_v2, DANGLING_EVIDENCE, DANGLING_PARENT, NULL_PARENT, SEED,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A fresh pool and one connection whose `search_path` is `schema`, owned by
/// this test. The pool is returned so it outlives the connection.
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

async fn migrate_to(
  conn: &PgConnection,
  dir: &str,
  registry: &SchemaRegistry,
  name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let file = run_generate(registry, dir, name, Dialect::Postgres)?.ok_or("no migration")?;
  assert_eq!(run_migrate(conn, dir, Dialect::Postgres).await?, 1);
  Ok(std::fs::read_to_string(format!("{dir}/{file}"))?)
}

/// Asserts `sql` fails with a foreign-key violation (SQLSTATE 23503).
async fn refused(conn: &PgConnection, sql: &str) -> TestResult {
  match conn.execute_sql(sql, vec![]).await {
    Ok(_) => Err(format!("accepted: {sql}").into()),
    Err(e) if format!("{e:?}").contains("23503") || e.to_string().contains("foreign key") => Ok(()),
    Err(e) => Err(format!("unexpected error for {sql}: {e:?}").into()),
  }
}

async fn seed(conn: &PgConnection) -> TestResult {
  for insert in SEED {
    conn.execute_sql(insert, vec![]).await?;
  }
  Ok(())
}

#[tokio::test]
async fn composite_fks_migrate_and_hold_on_postgres() -> TestResult {
  let (_db, conn) = pg_schema_conn("composite_fk_create").await?;
  let (_tmp, dir) = migrations_dir()?;
  let sql = migrate_to(&conn, &dir, &registry_v2(), "init").await?;
  assert!(
    sql.contains("ADD CONSTRAINT \"project_evidence_work_item_project_fk\" FOREIGN KEY"),
    "sql: {sql}"
  );
  seed(&conn).await?;
  refused(&conn, DANGLING_EVIDENCE).await?;
  refused(&conn, DANGLING_PARENT).await?;
  conn.execute_sql(NULL_PARENT, vec![]).await?;

  refused(&conn, "DELETE FROM project_work_items WHERE id = 'w1'").await?;
  conn
    .execute_sql("DELETE FROM project_work_items WHERE id = 'w2'", vec![])
    .await?;
  conn
    .execute_sql("DELETE FROM project_work_items WHERE id = 'w1'", vec![])
    .await?;
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_evidence").await?,
    0
  );
  Ok(())
}

#[tokio::test]
async fn adding_a_composite_fk_attaches_a_constraint_on_postgres() -> TestResult {
  let (_db, conn) = pg_schema_conn("composite_fk_add").await?;
  let (_tmp, dir) = migrations_dir()?;
  migrate_to(&conn, &dir, &registry_v1(), "init").await?;
  seed(&conn).await?;
  let sql = migrate_to(&conn, &dir, &registry_v2(), "evidence_fk").await?;
  assert!(
    sql.contains(
      "ALTER TABLE \"project_evidence\" ADD CONSTRAINT \"project_evidence_work_item_project_fk\""
    ),
    "sql: {sql}"
  );
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_evidence").await?,
    1
  );
  refused(&conn, DANGLING_EVIDENCE).await?;
  Ok(())
}
