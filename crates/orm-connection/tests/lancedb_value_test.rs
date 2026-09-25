#![cfg(feature = "lancedb")]

use std::{error::Error, path::PathBuf};
use toolu_orm_connection::{LanceConnection, LanceValueError, to_duckdb_params};
use toolu_orm_core::value::{JsonStorage, Value};

fn extension() -> Result<PathBuf, Box<dyn Error>> {
  Ok(
    std::env::var_os("LANCE_EXTENSION_PATH")
      .map(PathBuf::from)
      .ok_or("LANCE_EXTENSION_PATH is required for real Lance value tests")?,
  )
}

#[test]
fn prepared_insert_stores_each_portable_scalar_type() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let namespace = LanceConnection::open(extension()?)?.attach(directory.path(), "data")?;
  namespace.connection().execute(
    "CREATE TABLE data.main.scalars (id BIGINT, integer_value BIGINT, real_value DOUBLE, text_value VARCHAR, boolean_value BOOLEAN, nullable_text VARCHAR, blob_value BLOB)",
    [],
  )?;

  let values = to_duckdb_params(&[
    Value::Integer(1),
    Value::Integer(-7),
    Value::Real(2.5),
    Value::Text("O'Reilly; --".into()),
    Value::Boolean(true),
    Value::Null,
    Value::Blob(vec![0, 39, 255]),
  ])?;
  let mut insert = namespace
    .connection()
    .prepare("INSERT INTO scalars VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
  assert_eq!(insert.execute(duckdb::params_from_iter(values.iter()))?, 1);

  let stored: (i64, f64, String, bool, Option<String>, Vec<u8>) = namespace
    .connection()
    .query_row(
      "SELECT integer_value, real_value, text_value, boolean_value, nullable_text, blob_value FROM scalars WHERE id = 1",
      [],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    )?;
  assert_eq!(
    stored,
    (-7, 2.5, "O'Reilly; --".into(), true, None, vec![0, 39, 255])
  );
  assert!(to_duckdb_params(&[])?.is_empty());
  Ok(())
}

#[test]
fn prepared_filters_treat_quoted_text_null_and_binary_as_data() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let namespace = LanceConnection::open(extension()?)?.attach(directory.path(), "data")?;
  namespace.connection().execute(
    "CREATE TABLE data.main.items (id BIGINT, label VARCHAR, note VARCHAR, payload BLOB)",
    [],
  )?;
  let quoted = "x'; DROP TABLE items; --";
  let input = to_duckdb_params(&[
    Value::Integer(1),
    Value::Text(quoted.into()),
    Value::Null,
    Value::Blob(vec![0, 39, 255]),
  ])?;
  namespace
    .connection()
    .prepare("INSERT INTO items VALUES (?1, ?2, ?3, ?4)")?
    .execute(duckdb::params_from_iter(input.iter()))?;
  let predicate = to_duckdb_params(&[
    Value::Text(quoted.into()),
    Value::Blob(vec![0, 39, 255]),
    Value::Null,
  ])?;
  let matched: i64 = namespace
    .connection()
    .prepare("SELECT count(*) FROM items WHERE label = ?1 AND payload = ?2 AND note IS NOT DISTINCT FROM ?3")?
    .query_row(duckdb::params_from_iter(predicate.iter()), |row| row.get(0))?;
  assert_eq!(matched, 1);
  assert_eq!(namespace.list_tables()?, vec!["items"]);
  Ok(())
}

#[test]
fn deferred_values_fail_conversion_before_a_prepared_write() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let namespace = LanceConnection::open(extension()?)?.attach(directory.path(), "data")?;
  namespace.connection().execute(
    "CREATE TABLE data.main.items (id BIGINT, label VARCHAR)",
    [],
  )?;
  namespace
    .connection()
    .execute("INSERT INTO items VALUES (1, 'keep')", [])?;
  let insert = namespace
    .connection()
    .prepare("INSERT INTO items VALUES (?1, ?2)")?;

  let deferred = [
    ("TimestampEpoch", Value::TimestampEpoch(0)),
    (
      "TimestampText",
      Value::TimestampText("2026-09-25T00:00:00Z".into()),
    ),
    (
      "Json",
      Value::Json {
        text: "{}".into(),
        storage: JsonStorage::Json,
      },
    ),
    (
      "Uuid",
      Value::Uuid("00000000-0000-0000-0000-000000000000".into()),
    ),
    ("Numeric", Value::Numeric("1.25".into())),
  ];
  for (kind, value) in deferred {
    let error = to_duckdb_params(&[Value::Integer(2), value])
      .err()
      .ok_or("deferred value unexpectedly converted")?;
    assert!(matches!(error, LanceValueError::Unsupported { kind: actual } if actual == kind));
  }

  let count: i64 = namespace
    .connection()
    .query_row("SELECT count(*) FROM items", [], |row| row.get(0))?;
  assert_eq!(count, 1);
  let label: String =
    namespace
      .connection()
      .query_row("SELECT label FROM items WHERE id = 1", [], |row| row.get(0))?;
  assert_eq!(label, "keep");
  drop(insert);
  Ok(())
}
