use std::error::Error;
use toolu_orm_connection::{DbConnectionBlocking, DbError, LanceDbConnection};
use toolu_orm_core::{
  error::DbCoreError,
  row::{FromLanceValue, FromRow, LanceRow},
};

use super::support::session;

struct Field<T>(T);
impl<T: FromLanceValue> FromRow for Field<T> {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["value"];
  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    Ok(Self(row.get_typed("value")?))
  }
}

fn value<T: FromLanceValue>(
  conn: &LanceDbConnection,
  expression: &str,
) -> Result<T, Box<dyn Error>> {
  let sql = format!("SELECT {expression} AS value FROM scalars LIMIT 1");
  let mut rows: Vec<Field<T>> = DbConnectionBlocking::query_map(conn, &sql, vec![])?;
  Ok(rows.pop().ok_or("no scalar row")?.0)
}

#[test]
fn every_optional_scalar_accepts_null_and_present_values() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  assert_eq!(value::<Option<i64>>(&conn, "42::BIGINT")?, Some(42));
  assert_eq!(value::<Option<f64>>(&conn, "1.25::DOUBLE")?, Some(1.25));
  assert_eq!(value::<Option<bool>>(&conn, "false")?, Some(false));
  assert_eq!(value::<Option<String>>(&conn, "''")?, Some(String::new()));
  assert_eq!(value::<Option<Vec<u8>>>(&conn, "''::BLOB")?, Some(vec![]));
  assert_eq!(value::<Option<i64>>(&conn, "NULL::BIGINT")?, None);
  assert_eq!(value::<Option<f64>>(&conn, "NULL::DOUBLE")?, None);
  assert_eq!(value::<Option<bool>>(&conn, "NULL::BOOLEAN")?, None);
  assert_eq!(value::<Option<String>>(&conn, "NULL::VARCHAR")?, None);
  assert_eq!(value::<Option<Vec<u8>>>(&conn, "NULL::BLOB")?, None);
  Ok(())
}

#[test]
fn float_widens_and_nonfinite_doubles_remain_floats() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  assert_eq!(value::<f64>(&conn, "1.25::FLOAT")?, 1.25);
  assert!(value::<f64>(&conn, "'NaN'::DOUBLE")?.is_nan());
  assert_eq!(value::<f64>(&conn, "'Infinity'::DOUBLE")?, f64::INFINITY);
  assert_eq!(
    value::<f64>(&conn, "'-Infinity'::DOUBLE")?,
    f64::NEG_INFINITY
  );
  assert!(value::<f64>(&conn, "'-0.0'::DOUBLE")?.is_sign_negative());
  Ok(())
}

#[test]
fn required_null_and_optional_mismatches_never_coerce() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  let failures = [
    (value::<f64>(&conn, "NULL"), "f64"),
    (value::<String>(&conn, "NULL").map(|_| 0.0), "String"),
    (value::<bool>(&conn, "NULL").map(|_| 0.0), "bool"),
    (value::<Vec<u8>>(&conn, "NULL").map(|_| 0.0), "Vec<u8>"),
    (
      value::<Option<i64>>(&conn, "'secret payload'").map(|_| 0.0),
      "Option<i64>",
    ),
    (
      value::<Option<f64>>(&conn, "true").map(|_| 0.0),
      "Option<f64>",
    ),
    (
      value::<Option<bool>>(&conn, "1").map(|_| 0.0),
      "Option<bool>",
    ),
    (
      value::<Option<Vec<u8>>>(&conn, "'secret payload'").map(|_| 0.0),
      "Option<alloc::vec::Vec<u8>>",
    ),
  ];
  for (result, expected) in failures {
    let error = result.err().ok_or("invalid field accepted")?;
    let native = error.downcast_ref::<DbError>().ok_or("wrong error type")?;
    assert!(
      matches!(native, DbError::RowMapping(message) if message.contains("value") && message.contains(expected) && !message.contains("secret payload")),
      "{error}"
    );
  }
  let rows: Vec<Field<i64>> =
    DbConnectionBlocking::query_map(&conn, "SELECT text FROM scalars WHERE false", vec![])?;
  assert!(rows.is_empty());
  Ok(())
}
