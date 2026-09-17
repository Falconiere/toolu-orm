//! Recreating either table against a live in-memory libsql database: the
//! triggers come back and the index stays synchronized afterwards.
//!
//! The content-table case is the one #85 left open — its rebuild is
//! create-staging → copy → `DROP TABLE` → rename, and `DROP TABLE` takes every
//! trigger attached to the table with it.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::schema::SchemaRegistry;

use schema::{
  connect, insert_heron, integrity_check, matches, memories, memory_fts_unsynced, migrations_dir,
  registry, scalar, seed, trigger_count, TestResult,
};

#[tokio::test]
async fn a_tokenizer_change_recreates_the_table_and_its_triggers() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(
    &registry("porter unicode61", false),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  seed(&conn).await?;
  // The porter stemmer indexes "running" under its stem, so a search for the
  // stem hits; plain unicode61 indexes the word as written and it will not.
  assert_eq!(matches(&conn, "run").await?, 1);

  assert_eq!(
    run_generate(
      &registry("unicode61", false),
      &dir,
      "retokenize",
      Dialect::Sqlite
    )?
    .as_deref(),
    Some("0002_retokenize.sql")
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  // The table came back with the new tokenizer, its triggers, and its content.
  assert_eq!(trigger_count(&conn).await?, 3);
  assert_eq!(matches(&conn, "run").await?, 0);
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  integrity_check(&conn).await?;

  // And is still synchronized afterwards.
  insert_heron(&conn).await?;
  conn
    .execute_batch("DELETE FROM memories WHERE id = 1;")
    .await?;
  assert_eq!(matches(&conn, "heron").await?, 1);
  assert_eq!(matches(&conn, "zebrafish").await?, 0);
  integrity_check(&conn).await?;
  Ok(())
}

#[tokio::test]
async fn a_content_table_rebuild_puts_the_triggers_back() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(
    &registry("porter unicode61", false),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  seed(&conn).await?;

  // NOT NULL on an existing column is a change SQLite can only apply by
  // rebuilding the table, which drops every trigger attached to it.
  assert_eq!(
    run_generate(
      &registry("porter unicode61", true),
      &dir,
      "tighten",
      Dialect::Sqlite
    )?
    .as_deref(),
    Some("0002_tighten.sql")
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  assert_eq!(trigger_count(&conn).await?, 3);
  assert_eq!(scalar(&conn, "SELECT count(*) FROM memories").await?, 2);
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  integrity_check(&conn).await?;

  insert_heron(&conn).await?;
  conn
    .execute_batch("UPDATE memories SET body = 'a calm morning walk' WHERE id = 1;")
    .await?;
  conn
    .execute_batch("DELETE FROM memories WHERE id = 2;")
    .await?;
  assert_eq!(matches(&conn, "heron").await?, 1);
  assert_eq!(matches(&conn, "morning").await?, 1);
  assert_eq!(matches(&conn, "zebrafish").await?, 0);
  assert_eq!(matches(&conn, "afternoon").await?, 0);
  integrity_check(&conn).await?;
  Ok(())
}

#[tokio::test]
async fn removing_the_declaration_leaves_the_index_without_triggers() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  run_generate(
    &registry("porter unicode61", false),
    &dir,
    "init",
    Dialect::Sqlite,
  )?;
  run_migrate(&conn, &dir, Dialect::Sqlite).await?;
  seed(&conn).await?;

  let unmanaged = SchemaRegistry::from_tables(vec![
    memories(false),
    memory_fts_unsynced("porter unicode61"),
  ]);
  assert_eq!(
    run_generate(&unmanaged, &dir, "unmanage", Dialect::Sqlite)?.as_deref(),
    Some("0002_unmanage.sql")
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  assert_eq!(trigger_count(&conn).await?, 0);
  // Already-indexed rows stay searchable; new ones are the application's job.
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  insert_heron(&conn).await?;
  assert_eq!(matches(&conn, "heron").await?, 0);
  Ok(())
}
