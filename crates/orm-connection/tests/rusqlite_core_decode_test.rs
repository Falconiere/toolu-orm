//! `toolu_orm_core::row::from_rusqlite_row` against real rusqlite rows.
//!
//! The mirror of `libsql_core_decode_test`. Gated on nothing but `rusqlite`, so
//! it compiles in whatever shape the lane gives orm-core; the rusqlite-only lane
//! runs it on the single-driver shape, where `FromRow` exposes `from_row` and
//! the helper has to forward to that instead of `from_rusqlite_row`.
#![cfg(feature = "rusqlite")]

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::from_rusqlite_row;
use toolu_orm_macros::FromRow;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, PartialEq)]
struct Person {
  id: String,
  age: Option<i64>,
}

/// One NULL age and one present age, so decoding has to distinguish them.
const SETUP: &str = "CREATE TABLE people (id TEXT PRIMARY KEY, age INTEGER); \
   INSERT INTO people (id, age) VALUES ('p1', NULL); \
   INSERT INTO people (id, age) VALUES ('p2', 30);";

fn people_conn() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(SETUP)?;
  Ok(conn)
}

#[test]
fn from_rusqlite_row_decodes_real_rows_with_null_as_none() -> TestResult {
  let conn = people_conn()?;
  let mut stmt = conn.prepare("SELECT id, age FROM people ORDER BY id")?;
  let mut rows = stmt.query([])?;

  let mut people = Vec::new();
  while let Some(row) = rows.next()? {
    people.push(from_rusqlite_row::<Person>(row)?);
  }

  assert_eq!(
    people,
    vec![
      Person {
        id: "p1".into(),
        age: None
      },
      Person {
        id: "p2".into(),
        age: Some(30)
      },
    ]
  );
  Ok(())
}

#[test]
fn from_rusqlite_row_missing_column_is_row_mapping() -> TestResult {
  let conn = people_conn()?;
  let mut stmt = conn.prepare("SELECT id FROM people")?;
  let mut rows = stmt.query([])?;
  let Some(row) = rows.next()? else {
    return Err("expected at least one row".into());
  };

  let Err(DbCoreError::RowMapping(msg)) = from_rusqlite_row::<Person>(row) else {
    return Err("expected DbCoreError::RowMapping for the absent age column".into());
  };
  assert!(msg.contains("column 1 (age)"), "unexpected message: {msg}");
  Ok(())
}
