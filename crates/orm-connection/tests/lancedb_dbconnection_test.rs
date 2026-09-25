#![cfg(feature = "lancedb")]

use std::{error::Error, path::Path, sync::Arc};
use toolu_orm_connection::{
  DbConnection, DbConnectionBlocking, DbError, LanceConnection, LanceDbConnection,
};
use toolu_orm_core::{
  error::DbCoreError,
  row::{FromRow, LanceRow},
  value::{JsonStorage, Value},
};

#[derive(Debug, PartialEq)]
struct Item {
  id: i64,
  label: String,
}

impl FromRow for Item {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "label"];

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    let id_value = row.get("id")?;
    let id = if let Value::Integer(id) = id_value {
      *id
    } else {
      return Err(DbCoreError::RowMapping(format!(
        "id expected BIGINT, got {id_value:?}"
      )));
    };
    let label_value = row.get("label")?;
    let label = if let Value::Text(label) = label_value {
      label.clone()
    } else {
      return Err(DbCoreError::RowMapping(format!(
        "label expected VARCHAR, got {label_value:?}"
      )));
    };
    Ok(Self { id, label })
  }
}

struct NoLanceDecoder;

impl FromRow for NoLanceDecoder {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id"];
}

#[derive(Debug)]
struct Count(i64);

impl FromRow for Count {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["total"];

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    let value = row.get("total")?;
    if let Value::Integer(total) = value {
      Ok(Self(*total))
    } else {
      Err(DbCoreError::RowMapping(format!(
        "total expected BIGINT, got {value:?}"
      )))
    }
  }
}

fn extension() -> Result<std::path::PathBuf, Box<dyn Error>> {
  Ok(
    std::env::var_os("LANCE_EXTENSION_PATH")
      .map(std::path::PathBuf::from)
      .ok_or("LANCE_EXTENSION_PATH is required for real Lance connection tests")?,
  )
}

fn session(directory: &Path) -> Result<LanceDbConnection, Box<dyn Error>> {
  Ok(LanceDbConnection::from_namespace(
    LanceConnection::open(extension()?)?.attach(directory, "data")?,
  ))
}

#[tokio::test]
async fn async_bound_write_and_typed_read_keep_sql_punctuation_as_data()
-> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let conn = session(directory.path())?;
  DbConnection::execute_batch(&conn, "CREATE TABLE items (id BIGINT, label VARCHAR)").await?;
  let label = "x'; DROP TABLE items; --";
  assert_eq!(
    DbConnection::execute_sql(
      &conn,
      "INSERT INTO items VALUES (?1, ?2)",
      vec![Value::Integer(7), Value::Text(label.into())]
    )
    .await?,
    1
  );
  let rows: Vec<Item> = DbConnection::query_map(
    &conn,
    "SELECT id, label FROM items WHERE id = ?1",
    vec![Value::Integer(7)],
  )
  .await?;
  assert_eq!(
    rows,
    vec![Item {
      id: 7,
      label: label.into()
    }]
  );
  let absent: Vec<Item> = DbConnection::query_map(
    &conn,
    "SELECT id, label FROM items WHERE id = ?1",
    vec![Value::Integer(8)],
  )
  .await?;
  assert!(absent.is_empty());
  Ok(())
}

#[tokio::test]
async fn batch_create_persists_after_reopen_and_reports_unsupported_ddl()
-> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let conn = session(directory.path())?;
  DbConnection::execute_batch(
    &conn,
    "CREATE TABLE first_table (id BIGINT); CREATE TABLE second_table (label VARCHAR);",
  )
  .await?;
  let error = DbConnection::execute_batch(
    &conn,
    "CREATE TABLE unsupported_table (id BIGINT PRIMARY KEY)",
  )
  .await
  .err()
  .ok_or("unsupported Lance DDL succeeded")?;
  assert!(matches!(error, DbError::Query(message) if message.contains("PRIMARY KEY")));
  drop(conn);
  let reopened = LanceConnection::open(extension()?)?.attach(directory.path(), "again")?;
  assert_eq!(reopened.list_tables()?, vec!["first_table", "second_table"]);
  Ok(())
}

#[test]
fn blocking_methods_run_without_runtime_and_preserve_mapping_errors() -> Result<(), Box<dyn Error>>
{
  let directory = tempfile::tempdir()?;
  let conn = session(directory.path())?;
  DbConnectionBlocking::execute_batch(&conn, "CREATE TABLE items (id BIGINT, label VARCHAR)")?;
  assert_eq!(
    DbConnectionBlocking::execute_sql(
      &conn,
      "INSERT INTO items VALUES (?1, ?2)",
      vec![Value::Integer(1), Value::Text("keep".into())]
    )?,
    1
  );
  let rows: Vec<Item> =
    DbConnectionBlocking::query_map(&conn, "SELECT id, label FROM items", vec![])?;
  assert_eq!(
    rows,
    vec![Item {
      id: 1,
      label: "keep".into()
    }]
  );

  let deferred = DbConnectionBlocking::execute_sql(
    &conn,
    "INSERT INTO items VALUES (?1, ?2)",
    vec![
      Value::Integer(2),
      Value::Json {
        text: "{}".into(),
        storage: JsonStorage::Json,
      },
    ],
  )
  .err()
  .ok_or("deferred bind succeeded")?;
  assert!(matches!(deferred, DbError::Query(message) if message.contains("LanceUnsupportedValue")));
  let missing = DbConnectionBlocking::query_map::<Item>(&conn, "SELECT id FROM items", vec![])
    .err()
    .ok_or("missing column decoded")?;
  assert!(matches!(missing, DbError::RowMapping(message) if message.contains("label")));
  let unimplemented =
    DbConnectionBlocking::query_map::<NoLanceDecoder>(&conn, "SELECT id FROM items", vec![])
      .err()
      .ok_or("missing decoder succeeded")?;
  assert!(matches!(unimplemented, DbError::RowMapping(message) if message.contains("Lance")));
  let unsupported =
    DbConnectionBlocking::query_map::<Count>(&conn, "SELECT TRUE AS total FROM items", vec![])
      .err()
      .ok_or("unsupported result type decoded")?;
  assert!(
    matches!(unsupported, DbError::RowMapping(message) if message.contains("total") && message.contains("Boolean"))
  );
  let ambiguous = LanceRow::from_columns(vec![
    ("ID".into(), Value::Integer(1)),
    ("id".into(), Value::Integer(2)),
  ])
  .err()
  .ok_or("ambiguous result names were accepted")?;
  assert!(
    matches!(ambiguous, DbCoreError::RowMapping(message) if message == "ambiguous Lance result column id")
  );
  let rows: Vec<Count> =
    DbConnectionBlocking::query_map(&conn, "SELECT count(*) AS total FROM items", vec![])?;
  assert_eq!(rows.first().map(|row| row.0), Some(1));
  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_async_writes_serialize_on_one_duckdb_connection() -> Result<(), Box<dyn Error>>
{
  let directory = tempfile::tempdir()?;
  let conn = Arc::new(session(directory.path())?);
  DbConnection::execute_batch(&*conn, "CREATE TABLE items (id BIGINT, label VARCHAR)").await?;
  let mut tasks = Vec::new();
  for start in [0_i64, 100_i64] {
    let shared = Arc::clone(&conn);
    tasks.push(tokio::spawn(async move {
      for id in start..start + 10 {
        DbConnection::execute_sql(
          &*shared,
          "INSERT INTO items VALUES (?1, ?2)",
          vec![Value::Integer(id), Value::Text("saved".into())],
        )
        .await?;
      }
      Ok::<_, DbError>(())
    }));
  }
  for task in tasks {
    task.await??;
  }
  let rows: Vec<Count> =
    DbConnection::query_map(&*conn, "SELECT count(*) AS total FROM items", vec![]).await?;
  assert_eq!(rows.first().map(|row| row.0), Some(20));
  Ok(())
}
