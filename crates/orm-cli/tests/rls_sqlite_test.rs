//! A schema with row security still migrates on SQLite: `generate` writes the
//! Postgres-only operations as comments, `migrate` applies the file, and the
//! table exists without any policy machinery — there is none to have.

#[path = "fixtures/rls_registry.rs"]
pub mod rls_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;

use rls_registry::{migrations_dir, registry_v1, registry_v2};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn column_count(conn: &LibsqlConnection) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query("SELECT count(*) FROM pragma_table_info('docs')", ())
    .await?;
  let row = rows.next().await?.ok_or("count returned no row")?;
  Ok(row.get::<i64>(0)?)
}

#[tokio::test]
async fn row_security_renders_as_comments_and_migrates() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;

  let file = run_generate(&registry_v1(), &dir, "init", Dialect::Sqlite)?.ok_or("no migration")?;
  let sql = std::fs::read_to_string(format!("{dir}/{file}"))?;
  assert!(
    sql.contains("CREATE TABLE IF NOT EXISTS \"docs\""),
    "sql: {sql}"
  );
  assert!(
    sql.contains("-- row level security on \"docs\" (Postgres only"),
    "sql: {sql}"
  );
  assert!(
    sql.contains("-- policy \"tenant_isolation\" on \"docs\" (Postgres only"),
    "sql: {sql}"
  );
  assert!(!sql.contains("CREATE POLICY"), "sql: {sql}");
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(column_count(&conn).await?, 3);

  let file =
    run_generate(&registry_v2(), &dir, "restrict", Dialect::Sqlite)?.ok_or("no migration")?;
  let sql = std::fs::read_to_string(format!("{dir}/{file}"))?;
  assert!(sql.contains("ADD COLUMN \"archived\""), "sql: {sql}");
  assert!(sql.contains("-- policy \"hide_archived\""), "sql: {sql}");
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(column_count(&conn).await?, 4);
  Ok(())
}
