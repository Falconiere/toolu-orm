//! Generate → migrate → search against in-memory libsql: the FTS5 table is
//! created by real DDL, answers a real `MATCH` query, and any attempt to
//! evolve it in place is refused before a migration file is written.

use toolu_orm_cli::generate::run_generate;
use toolu_orm_cli::migrate::run_migrate;
use toolu_orm_connection::{Database, DbConnection, LibsqlConnection};
use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn migrations_dir() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
  let tmp = tempfile::tempdir()?;
  let dir = tmp.path().join("migrations");
  std::fs::create_dir(&dir)?;
  let path = dir.to_str().ok_or("non-UTF8 path")?.to_owned();
  Ok((tmp, path))
}

fn memories() -> TableDef {
  let column = |name: &str, primary_key: bool| ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key,
    not_null: primary_key,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  };
  TableDef {
    name: "memories".to_owned(),
    columns: vec![column("id", true), column("body", false)],
    indexes: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }
}

fn memory_fts(tokenize: &str) -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize(tokenize)
    .build()
}

fn registry(tokenize: &str) -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![memories(), memory_fts(tokenize)])
}

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

async fn text(conn: &LibsqlConnection, sql: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("query returned no row")?;
  Ok(row.get::<String>(0)?)
}

/// Applies the initial migration and seeds two rows through the FTS5 table.
async fn seeded(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  run_generate(&registry("porter unicode61"), dir, "init", Dialect::Sqlite)?;
  run_migrate(&conn, dir, Dialect::Sqlite).await?;
  conn
    .execute_batch(
      "INSERT INTO memory_fts (memory_id, body) \
       VALUES ('m1', 'the runner was running through zebrafish valley'), \
              ('m2', 'a quiet afternoon of reading')",
    )
    .await?;
  Ok(conn)
}

#[tokio::test]
async fn migrate_creates_the_virtual_table() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  let file = run_generate(&registry("porter unicode61"), &dir, "init", Dialect::Sqlite)?;
  assert_eq!(file.as_deref(), Some("0001_init.sql"));
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  let ddl = text(
    &conn,
    "SELECT sql FROM sqlite_master WHERE name = 'memory_fts'",
  )
  .await?;
  assert!(ddl.contains("CREATE VIRTUAL TABLE"), "actual DDL: {ddl}");
  assert!(ddl.contains("USING \"fts5\""), "actual DDL: {ddl}");
  assert!(ddl.contains("UNINDEXED"), "actual DDL: {ddl}");
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE name = 'memories' AND type = 'table'"
    )
    .await?,
    1
  );
  Ok(())
}

#[tokio::test]
async fn the_created_table_answers_a_match_query() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = seeded(&dir).await?;

  // The porter tokenizer stems "running" to the "runner" stem class.
  assert_eq!(
    text(
      &conn,
      "SELECT memory_id FROM memory_fts WHERE memory_fts MATCH 'runner'"
    )
    .await?,
    "m1"
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'afternoon'"
    )
    .await?,
    1
  );
  Ok(())
}

#[tokio::test]
async fn the_unindexed_column_is_stored_but_not_searchable() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = seeded(&dir).await?;

  // "zebrafish" sits in the indexed body, "m1" only in the UNINDEXED column.
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'zebrafish'"
    )
    .await?,
    1
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'm1'"
    )
    .await?,
    0
  );
  assert_eq!(
    text(&conn, "SELECT memory_id FROM memory_fts WHERE rowid = 1").await?,
    "m1"
  );
  Ok(())
}

#[tokio::test]
async fn regenerating_the_same_schema_finds_no_change() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let _conn = seeded(&dir).await?;
  assert_eq!(
    run_generate(&registry("porter unicode61"), &dir, "noop", Dialect::Sqlite)?,
    None
  );
  Ok(())
}

#[tokio::test]
async fn changing_the_tokenizer_is_refused_without_writing_a_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let _conn = seeded(&dir).await?;

  let error = run_generate(&registry("unicode61"), &dir, "retokenize", Dialect::Sqlite)
    .expect_err("expected the virtual-table change to be refused");
  let message = error.to_string();
  assert!(
    message.contains("memory_fts")
      && message.contains("module arguments")
      && message.contains("content =")
      && message.contains("rebuild"),
    "unhelpful error: {message}"
  );
  assert!(
    !std::path::Path::new(&dir)
      .join("0002_retokenize.sql")
      .exists(),
    "a migration was written for a refused change"
  );
  Ok(())
}
