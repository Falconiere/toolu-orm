use std::error::Error;
use toolu_orm_connection::{DbConnection, DbConnectionBlocking, DbError};

use super::support::{Scalars, session};

#[tokio::test]
async fn typed_scalars_round_trip_through_blocking_and_async_sessions() -> Result<(), Box<dyn Error>>
{
  let (_directory, conn) = session()?;
  let sql = "SELECT * FROM scalars ORDER BY integer";
  let rows: Vec<Scalars> = DbConnectionBlocking::query_map(&conn, sql, vec![])?;
  assert_eq!(
    rows,
    vec![
      Scalars {
        integer: i64::MIN,
        real: -2.5,
        text: "O'Reilly — 東京".into(),
        boolean: true,
        blob: vec![0, 39, 255],
        nullable: None
      },
      Scalars {
        integer: i64::MAX,
        real: 0.0,
        text: "".into(),
        boolean: false,
        blob: vec![],
        nullable: Some("present".into())
      },
    ]
  );
  assert_eq!(
    DbConnection::query_map::<Scalars>(&conn, sql, vec![]).await?,
    rows
  );
  assert!(
    DbConnectionBlocking::query_map::<Scalars>(&conn, "SELECT * FROM scalars WHERE false", vec![])?
      .is_empty()
  );
  Ok(())
}

#[test]
fn missing_null_and_mismatched_columns_name_expected_rust_type() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  for (sql, column, expected) in [
    (
      "SELECT real, text, boolean, blob, nullable FROM scalars",
      "integer",
      "i64",
    ),
    (
      "SELECT NULL AS integer, real, text, boolean, blob, nullable FROM scalars",
      "integer",
      "i64",
    ),
    (
      "SELECT text AS integer, real, text, boolean, blob, nullable FROM scalars",
      "integer",
      "i64",
    ),
    (
      "SELECT integer, integer AS real, text, boolean, blob, nullable FROM scalars",
      "real",
      "f64",
    ),
    (
      "SELECT integer, real, boolean AS text, boolean, blob, nullable FROM scalars",
      "text",
      "String",
    ),
    (
      "SELECT integer, real, text, integer AS boolean, blob, nullable FROM scalars",
      "boolean",
      "bool",
    ),
    (
      "SELECT integer, real, text, boolean, text AS blob, nullable FROM scalars",
      "blob",
      "Vec<u8>",
    ),
    (
      "SELECT integer, real, text, boolean, blob FROM scalars",
      "nullable",
      "Option",
    ),
    (
      "SELECT integer, real, text, boolean, blob, integer AS nullable FROM scalars",
      "nullable",
      "Option",
    ),
  ] {
    let error = DbConnectionBlocking::query_map::<Scalars>(&conn, sql, vec![])
      .err()
      .ok_or("invalid scalar decoded")?;
    assert!(
      matches!(error, DbError::RowMapping(ref message) if message.contains(column) && message.contains(expected)),
      "{error}"
    );
  }
  Ok(())
}
