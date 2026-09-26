use std::error::Error;
use toolu_orm_connection::{DbConnectionBlocking, LanceConnection, LanceDbConnection};
use toolu_orm_core::{
  error::DbCoreError,
  row::{FromRow, LanceRow},
  value::Value,
};

#[derive(Debug, PartialEq)]
pub struct Scalars {
  pub integer: i64,
  pub real: f64,
  pub text: String,
  pub boolean: bool,
  pub blob: Vec<u8>,
  pub nullable: Option<String>,
}

impl FromRow for Scalars {
  const REQUIRED_COLUMNS: &'static [&'static str] =
    &["integer", "real", "text", "boolean", "blob", "nullable"];

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    Ok(Self {
      integer: row.get_typed("integer")?,
      real: row.get_typed("real")?,
      text: row.get_typed("text")?,
      boolean: row.get_typed("boolean")?,
      blob: row.get_typed("blob")?,
      nullable: row.get_typed("nullable")?,
    })
  }
}

pub fn session() -> Result<(tempfile::TempDir, LanceDbConnection), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let conn = LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(directory.path(), "data")?,
  );
  DbConnectionBlocking::execute_batch(
    &conn,
    "CREATE TABLE scalars (integer BIGINT, real DOUBLE, text VARCHAR, boolean BOOLEAN, blob BLOB, nullable VARCHAR)",
  )?;
  for (integer, real, text, boolean, blob, nullable) in [
    (
      i64::MIN,
      -2.5,
      "O'Reilly — 東京",
      true,
      vec![0, 39, 255],
      Value::Null,
    ),
    (
      i64::MAX,
      0.0,
      "",
      false,
      vec![],
      Value::Text("present".into()),
    ),
  ] {
    DbConnectionBlocking::execute_sql(
      &conn,
      "INSERT INTO scalars VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      vec![
        Value::Integer(integer),
        Value::Real(real),
        Value::Text(text.into()),
        Value::Boolean(boolean),
        Value::Blob(blob),
        nullable,
      ],
    )?;
  }
  Ok((directory, conn))
}

#[derive(Debug)]
pub struct Integer(pub i64);
impl FromRow for Integer {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["value"];
  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    let typed: i64 = row.get_typed("value")?;
    assert_eq!(row.get("VALUE")?, &Value::Integer(typed));
    Ok(Self(typed))
  }
}
