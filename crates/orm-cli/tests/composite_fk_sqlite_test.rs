//! Table-level composite foreign keys declared with `#[foreign_key(...)]`,
//! generated, migrated and enforced on real in-memory libsql.

#[path = "fixtures/composite_fk_registry.rs"]
pub mod composite_fk_registry;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;

use composite_fk_registry::{
  migrations_dir, registry_v1, registry_v2, DANGLING_EVIDENCE, DANGLING_PARENT, NULL_PARENT, SEED,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = Database::init_local(":memory:").await?.connect()?;
  exec(&conn, "PRAGMA foreign_keys = ON").await?;
  Ok(conn)
}

async fn exec(conn: &LibsqlConnection, sql: &str) -> TestResult {
  conn.inner_conn().execute(sql, ()).await?;
  Ok(())
}

async fn count(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("count returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// Asserts `sql` fails with SQLite's foreign-key error.
async fn refused(conn: &LibsqlConnection, sql: &str) -> TestResult {
  match conn.inner_conn().execute(sql, ()).await {
    Ok(_) => Err(format!("accepted: {sql}").into()),
    Err(e) if e.to_string().contains("FOREIGN KEY constraint failed") => Ok(()),
    Err(e) => Err(format!("unexpected error for {sql}: {e}").into()),
  }
}

/// Generates `registry` as one migration and applies it; returns its SQL.
async fn migrate_to(
  conn: &LibsqlConnection,
  dir: &str,
  registry: &toolu_orm_core::schema::SchemaRegistry,
  name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let file = run_generate(registry, dir, name, Dialect::Sqlite)?.ok_or("no migration")?;
  assert_eq!(run_migrate(conn, dir, Dialect::Sqlite).await?, 1);
  Ok(std::fs::read_to_string(format!("{dir}/{file}"))?)
}

async fn seeded_v2(
) -> Result<(tempfile::TempDir, LibsqlConnection, String), Box<dyn std::error::Error>> {
  let (tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  let sql = migrate_to(&conn, &dir, &registry_v2(), "init").await?;
  for insert in SEED {
    exec(&conn, insert).await?;
  }
  Ok((tmp, conn, sql))
}

#[tokio::test]
async fn composite_fks_render_and_refuse_dangling_pairs() -> TestResult {
  let (_tmp, conn, sql) = seeded_v2().await?;
  assert!(
    sql.contains(r#"FOREIGN KEY ("parent_work_item_id", "project_id") REFERENCES "project_work_items" ("id", "project_id")"#),
    "sql: {sql}"
  );
  assert!(
    sql.contains(r#"FOREIGN KEY ("work_item_id", "project_id") REFERENCES "project_work_items" ("id", "project_id") ON DELETE CASCADE"#),
    "sql: {sql}"
  );
  refused(&conn, DANGLING_EVIDENCE).await?;
  refused(&conn, DANGLING_PARENT).await?;
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_evidence").await?,
    1
  );
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_work_items").await?,
    2
  );
  Ok(())
}

#[tokio::test]
async fn null_member_skips_the_composite_fk() -> TestResult {
  let (_tmp, conn, _) = seeded_v2().await?;
  refused(&conn, DANGLING_PARENT).await?;
  exec(&conn, NULL_PARENT).await?;
  assert_eq!(
    count(
      &conn,
      "SELECT count(*) FROM project_work_items WHERE id = 'w4'"
    )
    .await?,
    1
  );
  Ok(())
}

#[tokio::test]
async fn no_action_refuses_the_parent_delete_and_cascade_removes_children() -> TestResult {
  let (_tmp, conn, _) = seeded_v2().await?;
  refused(&conn, "DELETE FROM project_work_items WHERE id = 'w1'").await?;
  exec(&conn, "DELETE FROM project_work_items WHERE id = 'w2'").await?;
  exec(&conn, "DELETE FROM project_work_items WHERE id = 'w1'").await?;
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_evidence").await?,
    0
  );
  Ok(())
}

#[tokio::test]
async fn adding_a_composite_fk_rebuilds_the_populated_table() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  migrate_to(&conn, &dir, &registry_v1(), "init").await?;
  for insert in SEED {
    exec(&conn, insert).await?;
  }
  exec(&conn, DANGLING_EVIDENCE).await?;
  exec(&conn, "DELETE FROM project_evidence WHERE id = 'e2'").await?;

  let sql = migrate_to(&conn, &dir, &registry_v2(), "evidence_fk").await?;
  assert!(
    sql.contains("CREATE TABLE \"_toolu_new_project_evidence\""),
    "sql: {sql}"
  );
  assert_eq!(
    count(&conn, "SELECT count(*) FROM project_evidence").await?,
    1
  );
  refused(&conn, DANGLING_EVIDENCE).await?;
  Ok(())
}
