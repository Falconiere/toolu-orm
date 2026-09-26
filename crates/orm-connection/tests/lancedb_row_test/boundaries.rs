use std::error::Error;
use toolu_orm_connection::{DbConnectionBlocking, DbError};

use super::support::{Integer, session};

#[test]
fn integer_widths_are_checked_and_aliases_are_case_insensitive() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  for (literal, kind, expected) in [
    ("-128", "TINYINT", -128),
    ("32767", "SMALLINT", 32767),
    ("-2147483648", "INTEGER", -2147483648),
    ("255", "UTINYINT", 255),
    ("65535", "USMALLINT", 65535),
    ("4294967295", "UINTEGER", 4294967295),
    ("9223372036854775807", "UBIGINT", i64::MAX),
    ("-9223372036854775808", "HUGEINT", i64::MIN),
    ("9223372036854775807", "UHUGEINT", i64::MAX),
  ] {
    let sql = format!("SELECT CAST({literal} AS {kind}) AS value FROM scalars LIMIT 1");
    let rows: Vec<Integer> = DbConnectionBlocking::query_map(&conn, &sql, vec![])?;
    assert_eq!(rows.first().map(|row| row.0), Some(expected));
  }
  for (literal, kind) in [
    ("18446744073709551615", "UBIGINT"),
    ("9223372036854775808", "HUGEINT"),
    ("-9223372036854775809", "HUGEINT"),
    ("340282366920938463463374607431768211455", "UHUGEINT"),
  ] {
    let sql = format!("SELECT CAST('{literal}' AS {kind}) AS value FROM scalars LIMIT 1");
    let error = DbConnectionBlocking::query_map::<Integer>(&conn, &sql, vec![])
      .err()
      .ok_or("overflow accepted")?;
    assert!(
      matches!(error, DbError::RowMapping(ref message) if message.contains("value") && message.contains("i64")),
      "{error}"
    );
  }
  Ok(())
}

#[test]
fn unsupported_results_and_duplicate_names_fail_explicitly() -> Result<(), Box<dyn Error>> {
  let (_directory, conn) = session()?;
  for expression in ["DATE '2026-09-26'", "CAST(1.25 AS DECIMAL(4,2))", "[1,2]"] {
    let sql = format!("SELECT {expression} AS value FROM scalars LIMIT 1");
    let error = DbConnectionBlocking::query_map::<Integer>(&conn, &sql, vec![])
      .err()
      .ok_or("unsupported result accepted")?;
    assert!(
      matches!(error, DbError::RowMapping(ref message) if message.contains("value") && message.contains("unsupported result type")),
      "{error}"
    );
  }
  let error = DbConnectionBlocking::query_map::<Integer>(
    &conn,
    "SELECT integer AS value, integer AS VALUE FROM scalars",
    vec![],
  )
  .err()
  .ok_or("duplicate alias accepted")?;
  assert!(
    matches!(error, DbError::RowMapping(ref message) if message.contains("ambiguous") && message.to_ascii_lowercase().contains("value")),
    "{error}"
  );
  Ok(())
}
