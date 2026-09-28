use std::{error::Error, path::PathBuf, time::SystemTime};
use toolu_orm::{
  connection::{DbConnectionBlocking, LanceConnection, LanceDbConnection},
  core::value::Value,
  FromRow,
};

#[derive(Debug, PartialEq, FromRow)]
pub struct Scalars {
  pub integer: i64,
  pub real: f64,
  pub text: String,
  pub boolean: bool,
  pub blob: Vec<u8>,
  pub nullable: Option<String>,
}

pub struct Directory(PathBuf);

impl Directory {
  /// Remove this test's data after its connection has been dropped.
  ///
  /// # Errors
  /// Returns the filesystem error if the directory cannot be removed.
  pub fn remove(self) -> std::io::Result<()> {
    std::fs::remove_dir_all(self.0)
  }
}

/// Create and seed an isolated, real Lance session.
///
/// # Errors
/// Propagates clock, filesystem, extension startup, and database errors.
pub fn session() -> Result<(Directory, LanceDbConnection), Box<dyn Error>> {
  let nonce = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_nanos();
  let path = std::env::temp_dir().join(format!("toolu-161-{}-{nonce}", std::process::id()));
  std::fs::create_dir(&path)?;
  let directory = Directory(path);
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let conn = LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(&directory.0, "data")?,
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
