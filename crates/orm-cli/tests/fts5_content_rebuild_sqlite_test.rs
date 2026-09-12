//! External-content FTS5: generate emits drop + recreate + rebuild, and a
//! live libsql database keeps searchable content after the retokenize migration.
//!
//! `content_rowid` must be an integer column (SQLite FTS5 rule), so the
//! content table uses an integer primary key.

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
  TableDef {
    name: "memories".to_owned(),
    columns: vec![
      ColumnDef {
        name: "id".to_owned(),
        column_type: ColumnType::Integer,
        primary_key: true,
        not_null: true,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
      ColumnDef {
        name: "body".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: false,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
  }
}

fn memory_fts(tokenize: &str) -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize(tokenize)
    .content("memories")
    .content_rowid("id")
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

#[tokio::test]
async fn retokenizing_external_content_fts_rebuilds_from_the_content_table() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;

  assert_eq!(
    run_generate(&registry("porter unicode61"), &dir, "init", Dialect::Sqlite)?.as_deref(),
    Some("0001_init.sql")
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  conn
    .execute_batch(
      "INSERT INTO memories (id, body) VALUES \
       (1, 'the athlete was running through zebrafish valley'), \
       (2, 'a quiet afternoon of reading');\n\
       INSERT INTO memory_fts(memory_fts) VALUES('rebuild');",
    )
    .await?;

  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'zebrafish'"
    )
    .await?,
    1
  );

  assert_eq!(
    run_generate(&registry("unicode61"), &dir, "retokenize", Dialect::Sqlite)?.as_deref(),
    Some("0002_retokenize.sql")
  );
  let sql = std::fs::read_to_string(std::path::Path::new(&dir).join("0002_retokenize.sql"))?;
  assert!(
    sql.contains("DROP TABLE IF EXISTS \"memory_fts\""),
    "generated migration missing drop: {sql}"
  );
  assert!(
    sql.contains("tokenize = 'unicode61'"),
    "generated migration missing new tokenizer: {sql}"
  );
  assert!(
    sql.contains("VALUES('rebuild')"),
    "generated migration missing rebuild: {sql}"
  );
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);

  let ddl = text(
    &conn,
    "SELECT sql FROM sqlite_master WHERE name = 'memory_fts'",
  )
  .await?;
  assert!(
    ddl.contains("tokenize = 'unicode61'"),
    "live DDL still has the old tokenizer: {ddl}"
  );
  assert!(
    !ddl.contains("porter"),
    "live DDL still mentions porter: {ddl}"
  );

  // Content table rows survived; rebuild made them searchable again.
  assert_eq!(scalar(&conn, "SELECT count(*) FROM memories").await?, 2);
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
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'afternoon'"
    )
    .await?,
    1
  );
  Ok(())
}
