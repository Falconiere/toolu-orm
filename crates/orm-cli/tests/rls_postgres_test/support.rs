//! Per-test schema connection, catalog reads, and the non-owner role the
//! enforcement test switches to.

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{DbConnection, PgConfig, PgConnection, PgDatabase};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::row::PgCountScalar;
use toolu_orm_core::schema::SchemaRegistry;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Opens a fresh pool and one connection whose `search_path` is a schema owned
/// by this test. The pool is returned so it outlives the connection.
pub async fn pg_schema_conn(
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

pub async fn count(conn: &impl DbConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows = conn.query_map::<PgCountScalar>(sql, vec![]).await?;
  Ok(rows.first().ok_or("count query returned no row")?.value)
}

/// `generate` then `migrate` for one registry version; returns the number of
/// migrations applied.
pub async fn generate_and_migrate(
  conn: &PgConnection,
  dir: &str,
  registry: &SchemaRegistry,
  name: &str,
) -> Result<u32, Box<dyn std::error::Error>> {
  run_generate(registry, dir, name, Dialect::Postgres)?;
  Ok(run_migrate(conn, dir, Dialect::Postgres).await?)
}

/// `(relrowsecurity, relforcerowsecurity)` for `docs` in the current schema.
pub async fn rls_flags(conn: &PgConnection) -> Result<(bool, bool), Box<dyn std::error::Error>> {
  let enabled = count(
    conn,
    "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
     WHERE n.nspname = current_schema() AND c.relname = 'docs' AND c.relrowsecurity",
  )
  .await?;
  let forced = count(
    conn,
    "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
     WHERE n.nspname = current_schema() AND c.relname = 'docs' AND c.relforcerowsecurity",
  )
  .await?;
  Ok((enabled == 1, forced == 1))
}

/// How many policies on `docs` match `predicate` (a `pg_policies` filter).
pub async fn policies_where(
  conn: &PgConnection,
  predicate: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
  count(
    conn,
    &format!(
      "SELECT count(*) FROM pg_policies \
       WHERE schemaname = current_schema() AND tablename = 'docs' AND ({predicate})"
    ),
  )
  .await
}

/// Creates `role` (NOLOGIN, idempotent) and grants it the current schema and
/// every table in it. The test user is a superuser and the table owner, so
/// both bypass row security; only this role sees the policies apply.
pub async fn grant_app_role(conn: &PgConnection, role: &str) -> TestResult {
  conn
    .execute_batch(&format!(
      "DO $$ BEGIN \
         IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{role}') THEN \
           CREATE ROLE {role} NOLOGIN; \
         END IF; \
       END $$; \
       GRANT USAGE ON SCHEMA {schema} TO {role}; \
       GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA {schema} TO {role}",
      schema = "rls_enforce"
    ))
    .await?;
  Ok(())
}
