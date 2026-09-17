// The generated FTS5 synchronization triggers, executed by the bundled
// rusqlite SQLite: the index tracks the content table with no manual rebuild.
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::DbConnection;
use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct ScalarRow {
  value: i64,
}

impl FromRow for ScalarRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let value: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { value })
  }
}

fn column(name: &str, column_type: ColumnType, primary_key: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type,
    primary_key,
    not_null: primary_key,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}

fn memories() -> TableDef {
  TableDef {
    name: "memories".to_owned(),
    columns: vec![
      column("id", ColumnType::Integer, true),
      column("body", ColumnType::Text, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
  }
}

fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("porter unicode61")
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build()
}

/// The whole generated schema: both tables, then the three triggers and the
/// rebuild, exactly as `generate` would write the migration.
fn schema_ddl() -> String {
  let fts = memory_fts();
  let sync = fts.fts5_sync.clone().unwrap_or_default();
  generate_sql_for(
    &[
      Operation::CreateTable { table: memories() },
      Operation::CreateTable { table: fts },
      Operation::CreateFts5SyncTriggers {
        table: "memory_fts".to_owned(),
        sync,
      },
    ],
    Dialect::Sqlite,
  )
}

/// A connection with that schema applied, statement chunk by statement chunk,
/// the way the migration runner applies one.
async fn migrated() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  let conn = RusqliteConnection::open_in_memory().await?;
  for chunk in schema_ddl().split("--> statement-breakpoint") {
    if chunk.trim().is_empty() {
      continue;
    }
    conn.execute_batch(chunk.trim()).await?;
  }
  Ok(conn)
}

async fn scalar(conn: &RusqliteConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows: Vec<ScalarRow> = conn.query_map(sql, vec![]).await?;
  Ok(rows.first().ok_or("no row")?.value)
}

async fn matches(conn: &RusqliteConnection, term: &str) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(
    conn,
    &format!("SELECT count(*) FROM memory_fts WHERE memory_fts MATCH '{term}'"),
  )
  .await
}

#[tokio::test]
async fn the_generated_ddl_creates_all_three_triggers() -> TestResult {
  let conn = migrated().await?;
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name LIKE 'toolu_fts5_%'"
    )
    .await?,
    3
  );
  Ok(())
}

#[tokio::test]
async fn an_insert_and_a_delete_reach_the_index_without_a_rebuild() -> TestResult {
  let conn = migrated().await?;
  conn
    .execute_batch(
      "INSERT INTO memories (id, body) VALUES \
       (1, 'the athlete was running through zebrafish valley'), \
       (2, 'a quiet afternoon of reading')",
    )
    .await?;
  assert_eq!(matches(&conn, "zebrafish").await?, 1);
  assert_eq!(matches(&conn, "afternoon").await?, 1);

  conn
    .execute_batch("DELETE FROM memories WHERE id = 2")
    .await?;
  assert_eq!(matches(&conn, "afternoon").await?, 0);
  assert_eq!(matches(&conn, "zebrafish").await?, 1);

  // FTS5's own verdict that the index matches the content table exactly.
  conn
    .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES('integrity-check')")
    .await?;
  Ok(())
}
