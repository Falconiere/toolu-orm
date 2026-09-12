//! Composite PRIMARY KEY and AUTOINCREMENT against real in-memory libsql.

use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn exec(conn: &LibsqlConnection, sql: &str) -> TestResult {
  conn.inner_conn().execute(sql, ()).await?;
  Ok(())
}

fn composite_table() -> TableDef {
  TableDef {
    name: "memory_tags".to_owned(),
    columns: vec![
      ColumnDef {
        name: "memory_id".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
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
        name: "tag".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
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
    ],
    indexes: vec![],
    primary_key: vec!["memory_id".to_owned(), "tag".to_owned()],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }
}

fn autoincrement_table() -> TableDef {
  TableDef {
    name: "retrieval_log".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Integer,
      primary_key: true,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: true,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }
}

#[tokio::test]
async fn composite_primary_key_ddl_runs_on_libsql() -> TestResult {
  let conn = connect().await?;
  let sql = generate_sql_for(
    &[Operation::CreateTable {
      table: composite_table(),
    }],
    Dialect::Sqlite,
  );
  exec(&conn, sql.trim_end_matches(';')).await?;
  exec(
    &conn,
    "INSERT INTO memory_tags (memory_id, tag) VALUES ('m1', 'a')",
  )
  .await?;
  let dup = exec(
    &conn,
    "INSERT INTO memory_tags (memory_id, tag) VALUES ('m1', 'a')",
  )
  .await;
  assert!(dup.is_err(), "duplicate composite key must fail");
  exec(
    &conn,
    "INSERT INTO memory_tags (memory_id, tag) VALUES ('m1', 'b')",
  )
  .await?;
  Ok(())
}

#[tokio::test]
async fn autoincrement_assigns_ids_on_libsql() -> TestResult {
  let conn = connect().await?;
  let sql = generate_sql_for(
    &[Operation::CreateTable {
      table: autoincrement_table(),
    }],
    Dialect::Sqlite,
  );
  exec(&conn, sql.trim_end_matches(';')).await?;
  exec(&conn, "INSERT INTO retrieval_log DEFAULT VALUES").await?;
  exec(&conn, "INSERT INTO retrieval_log DEFAULT VALUES").await?;
  let mut rows = conn
    .inner_conn()
    .query("SELECT id FROM retrieval_log ORDER BY id", ())
    .await?;
  let first = rows
    .next()
    .await?
    .ok_or("missing first row")?
    .get::<i64>(0)?;
  let second = rows
    .next()
    .await?
    .ok_or("missing second row")?
    .get::<i64>(0)?;
  assert_eq!(first, 1);
  assert_eq!(second, 2);
  Ok(())
}
