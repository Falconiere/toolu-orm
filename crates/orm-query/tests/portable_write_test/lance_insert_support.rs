use super::support::TestResult;
use toolu_orm_connection::{DbConnection, LanceConnection, LanceDbConnection};
use toolu_orm_core::{
  column::{Blob, Boolean, Integer, Real, Text},
  error::DbCoreError,
  query_column::Column,
  row::{FromRow, LanceRow},
  value::Value,
};

pub const ID: Column<Integer> = Column::new("insert_source", "id");
pub const LABEL: Column<Text> = Column::new("insert_source", "label");
pub const SCORE: Column<Real> = Column::new("insert_source", "score");
pub const LIVE: Column<Boolean> = Column::new("insert_source", "live");
pub const BYTES: Column<Blob> = Column::new("insert_source", "bytes");
pub const NOTE: Column<Text> = Column::new("insert_source", "note");
pub const TEXT: &str = "O'Brien ?1 $2; DROP TABLE insert_source; -- 東京";
pub const COLUMNS: &[&str] = &["id", "label", "score", "live", "bytes", "note"];

#[derive(Debug, PartialEq)]
pub struct Stored(pub Vec<Value>);

impl FromRow for Stored {
  const REQUIRED_COLUMNS: &'static [&'static str] = COLUMNS;

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    COLUMNS
      .iter()
      .map(|name| row.get(name).cloned())
      .collect::<Result<Vec<_>, _>>()
      .map(Self)
  }
}

pub fn open(directory: &std::path::Path) -> Result<LanceDbConnection, Box<dyn std::error::Error>> {
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  Ok(LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(directory, "data")?,
  ))
}

pub async fn setup(conn: &LanceDbConnection) -> TestResult {
  conn.execute_batch(
    "CREATE TABLE insert_source (id BIGINT, label VARCHAR, score DOUBLE, live BOOLEAN, bytes BLOB, note VARCHAR);
     CREATE TABLE insert_target (id BIGINT, label VARCHAR, score DOUBLE, live BOOLEAN, bytes BLOB, note VARCHAR)",
  ).await?;
  Ok(())
}

pub async fn rows(
  conn: &LanceDbConnection,
  target: bool,
) -> Result<Vec<Stored>, Box<dyn std::error::Error>> {
  let sql = if target {
    "SELECT id, label, score, live, bytes, note FROM insert_target ORDER BY id"
  } else {
    "SELECT id, label, score, live, bytes, note FROM insert_source ORDER BY id"
  };
  Ok(conn.query_map(sql, vec![]).await?)
}

pub fn expected(id: i64, empty: bool) -> Stored {
  Stored(vec![
    Value::Integer(id),
    Value::Text(if empty { "" } else { TEXT }.into()),
    Value::Real(if empty { 0.0 } else { -12.5 }),
    Value::Boolean(!empty),
    Value::Blob(if empty { vec![] } else { vec![0, 127, 255] }),
    Value::Null,
  ])
}
