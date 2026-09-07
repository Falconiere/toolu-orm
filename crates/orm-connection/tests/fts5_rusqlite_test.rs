// The generated `CREATE VIRTUAL TABLE` DDL, executed by the bundled rusqlite
// SQLite: it creates a searchable FTS5 table with an UNINDEXED column.
#![cfg(all(
  feature = "rusqlite",
  not(feature = "libsql"),
  not(feature = "postgres")
))]

use toolu_orm_connection::DbConnection;
use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::sql::generate_sql_for;

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

struct TextRow {
  value: String,
}

impl FromRow for TextRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let value: String = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { value })
  }
}

fn memory_fts_ddl() -> String {
  let table = Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("porter unicode61")
    .build();
  generate_sql_for(&[Operation::CreateTable { table }], Dialect::Sqlite)
}

/// A connection with the generated DDL applied and two rows indexed.
async fn seeded() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  let conn = RusqliteConnection::open_in_memory().await?;
  conn.execute_batch(&memory_fts_ddl()).await?;
  conn
    .execute_batch(
      "INSERT INTO memory_fts (memory_id, body) \
       VALUES ('m1', 'the runner was running through zebrafish valley'), \
              ('m2', 'a quiet afternoon of reading')",
    )
    .await?;
  Ok(conn)
}

async fn scalar(conn: &RusqliteConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let rows: Vec<ScalarRow> = conn.query_map(sql, vec![]).await?;
  Ok(rows.first().ok_or("no row")?.value)
}

#[tokio::test]
async fn generated_ddl_creates_a_virtual_table() -> Result<(), Box<dyn std::error::Error>> {
  let conn = seeded().await?;
  let rows: Vec<TextRow> = conn
    .query_map(
      "SELECT sql FROM sqlite_master WHERE name = 'memory_fts'",
      vec![],
    )
    .await?;
  let ddl = &rows.first().ok_or("memory_fts is missing")?.value;
  assert!(ddl.contains("CREATE VIRTUAL TABLE"), "actual DDL: {ddl}");
  assert!(ddl.contains("USING \"fts5\""), "actual DDL: {ddl}");
  Ok(())
}

#[tokio::test]
async fn match_finds_the_stemmed_row() -> Result<(), Box<dyn std::error::Error>> {
  let conn = seeded().await?;
  let rows: Vec<TextRow> = conn
    .query_map(
      "SELECT memory_id FROM memory_fts WHERE memory_fts MATCH 'runner'",
      vec![],
    )
    .await?;
  assert_eq!(rows.len(), 1);
  assert_eq!(rows.first().ok_or("no row")?.value, "m1");
  Ok(())
}

#[tokio::test]
async fn the_unindexed_column_is_stored_but_not_searchable()
-> Result<(), Box<dyn std::error::Error>> {
  let conn = seeded().await?;
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
      "SELECT count(*) FROM memory_fts WHERE memory_fts MATCH 'm2'"
    )
    .await?,
    0
  );
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM memory_fts").await?,
    2,
    "the UNINDEXED rows were not stored"
  );
  Ok(())
}
