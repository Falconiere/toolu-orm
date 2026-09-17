//! Generated FTS5 synchronization against a live in-memory libsql database:
//! writes to the content table reach the index with no manual `rebuild`.
//!
//! Recreating either table is the other half, in
//! `fts5_sync_recreate_sqlite_test.rs`.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{DbConnection, LibsqlConnection};
use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;

use schema::{
  connect, integrity_check, matches, memories, migrations_dir, registry, scalar, seed,
  trigger_count, TestResult,
};

/// The rowid the index has the matching row under, which for an external
/// content table is the value of its `content_rowid` column.
async fn indexed_rowid(
  conn: &LibsqlConnection,
  term: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT rowid FROM memory_fts WHERE memory_fts MATCH '{term}'"),
  )
  .await
}

#[tokio::test]
async fn content_mutations_stay_indexed_without_a_manual_rebuild() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  assert_eq!(
    run_generate(
      &registry("porter unicode61", false),
      &dir,
      "init",
      Dialect::Sqlite
    )?
    .as_deref(),
    Some("0001_init.sql")
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(trigger_count(&conn).await?, 3);

  // No `rebuild` here: the insert trigger is the only thing indexing this row.
  seed(&conn).await?;
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  assert_eq!(matches(&conn, "afternoon").await?, 1);
  integrity_check(&conn).await?;

  conn
    .execute_batch("UPDATE memories SET body = 'a calm morning walk' WHERE id = 1;")
    .await?;
  assert_eq!(matches(&conn, "zebrafish").await?, 0);
  assert_eq!(matches(&conn, "morning").await?, 1);
  integrity_check(&conn).await?;

  conn
    .execute_batch("DELETE FROM memories WHERE id = 2;")
    .await?;
  assert_eq!(matches(&conn, "afternoon").await?, 0);
  assert_eq!(matches(&conn, "morning").await?, 1);
  integrity_check(&conn).await?;
  Ok(())
}

#[tokio::test]
async fn an_unindexed_only_write_leaves_the_index_alone() -> TestResult {
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

  conn
    .execute_batch("UPDATE memories SET note = 'edited' WHERE id = 1;")
    .await?;
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  integrity_check(&conn).await?;
  Ok(())
}

#[tokio::test]
async fn a_rowid_change_moves_the_indexed_row_with_it() -> TestResult {
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
  assert_eq!(indexed_rowid(&conn, "zebrafish").await?, 1);

  conn
    .execute_batch("UPDATE memories SET id = 7 WHERE id = 1;")
    .await?;
  assert_eq!(indexed_rowid(&conn, "zebrafish").await?, 7);
  integrity_check(&conn).await?;
  Ok(())
}

#[tokio::test]
async fn an_invalid_declaration_writes_no_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let no_rowid = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("memories")
    .sync_content()
    .build();
  let registry = SchemaRegistry::from_tables(vec![memories(false), no_rowid]);
  let error = run_generate(&registry, &dir, "init", Dialect::Sqlite)
    .err()
    .map(|e| e.to_string())
    .unwrap_or_default();
  assert!(
    error.contains("cannot synchronize FTS5 table \"memory_fts\""),
    "unexpected outcome: {error}"
  );
  let written: Vec<_> = std::fs::read_dir(&dir)?.collect();
  assert!(
    written.is_empty(),
    "a refused schema still wrote {} entries",
    written.len()
  );
  Ok(())
}
