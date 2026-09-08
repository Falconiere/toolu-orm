//! `#[derive(FromRow)]` on the rusqlite-only shape, against a real database.
//!
//! The rusqlite counterpart of `libsql_derived_from_row_test`: this lane gives
//! `toolu-orm-core` rusqlite alone, so `FromRow` asks for a single
//! `from_row(&rusqlite::Row<'_>)`. Rows come out of an in-memory rusqlite
//! database with raw driver SQL, so a select can name fewer columns than the
//! struct declares — `select_for::<T>()` always selects `REQUIRED_COLUMNS`.

#[path = "fixtures/rusqlite_db.rs"]
pub mod db;

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_macros::FromRow;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, PartialEq)]
struct User {
  id: String,
  name: String,
  email: String,
  age: Option<i64>,
}

/// `#[from_row(with = "…")]` runs on the field's own decoded type, so it
/// normalizes or rejects a `String` rather than converting between types.
fn normalize_email(raw: String) -> Result<String, DbCoreError> {
  if raw.contains('@') {
    let mut normalized = raw;
    normalized.make_ascii_lowercase();
    Ok(normalized)
  } else {
    Err(DbCoreError::RowMapping(format!("{raw:?} is not an email")))
  }
}

#[derive(FromRow, Debug, PartialEq)]
struct Contact {
  id: String,
  #[from_row(with = "normalize_email")]
  email: String,
}

/// `u1` has a `NULL` age and a mixed-case email; `u2` has an age and an email
/// `normalize_email` rejects.
fn seeded() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = db::setup_db()?;
  conn.execute(
    "INSERT INTO users (id, name, email, age) VALUES ('u1', 'Ada', 'ADA@Example.COM', NULL)",
    (),
  )?;
  conn.execute(
    "INSERT INTO users (id, name, email, age) VALUES ('u2', 'Bob', 'not-an-email', 30)",
    (),
  )?;
  Ok(conn)
}

/// Decodes every row of `sql` through the derive, keeping per-row failures.
fn decode<T: FromRow>(
  conn: &rusqlite::Connection,
  sql: &str,
) -> Result<Vec<Result<T, DbCoreError>>, Box<dyn std::error::Error>> {
  let mut stmt = conn.prepare(sql)?;
  let mut rows = stmt.query([])?;
  let mut decoded = Vec::new();
  while let Some(row) = rows.next()? {
    decoded.push(T::from_row(row));
  }
  Ok(decoded)
}

fn row_mapping_message<T>(
  result: Result<T, DbCoreError>,
) -> Result<String, Box<dyn std::error::Error>> {
  match result {
    Err(DbCoreError::RowMapping(msg)) => Ok(msg),
    Err(other) => Err(format!("expected RowMapping, got {other:?}").into()),
    Ok(_) => Err("expected RowMapping, got a decoded row".into()),
  }
}

#[test]
fn derive_decodes_real_rusqlite_rows_with_null_as_none() -> TestResult {
  let conn = seeded()?;

  let decoded = decode::<User>(&conn, "SELECT id, name, email, age FROM users ORDER BY id")?;
  let users = decoded.into_iter().collect::<Result<Vec<_>, _>>()?;

  assert_eq!(
    users,
    vec![
      User {
        id: "u1".to_owned(),
        name: "Ada".to_owned(),
        email: "ADA@Example.COM".to_owned(),
        age: None,
      },
      User {
        id: "u2".to_owned(),
        name: "Bob".to_owned(),
        email: "not-an-email".to_owned(),
        age: Some(30),
      },
    ]
  );
  Ok(())
}

#[test]
fn derive_reports_required_columns_in_field_order() {
  assert_eq!(User::REQUIRED_COLUMNS, &["id", "name", "email", "age"]);
}

#[test]
fn derive_on_a_short_select_is_row_mapping_naming_the_column() -> TestResult {
  let conn = seeded()?;

  let mut decoded = decode::<User>(&conn, "SELECT id, name FROM users WHERE id = 'u1'")?;
  let Some(first) = decoded.pop() else {
    return Err("expected one row".into());
  };
  let msg = row_mapping_message(first)?;

  assert!(
    msg.contains("column 2 (email)"),
    "message must name the first missing index and column: {msg}"
  );
  Ok(())
}

/// rusqlite validates the index against the statement's column count, so an
/// absent *trailing nullable* column is an error here — unlike libsql, which
/// reads an out-of-range index as SQL `NULL` and decodes `None` (see
/// `libsql_derived_from_row_test`). Pinned so the difference is a documented
/// driver trait rather than an accident of whichever lane ran.
#[test]
fn derive_absent_trailing_nullable_column_is_rejected() -> TestResult {
  let conn = seeded()?;

  let mut decoded = decode::<User>(&conn, "SELECT id, name, email FROM users WHERE id = 'u2'")?;
  let Some(first) = decoded.pop() else {
    return Err("expected one row".into());
  };
  let msg = row_mapping_message(first)?;

  assert!(
    msg.contains("column 3 (age)"),
    "message must name the missing index and column: {msg}"
  );
  Ok(())
}

#[test]
fn derive_with_attribute_normalizes_then_rejects() -> TestResult {
  let conn = seeded()?;

  let mut decoded = decode::<Contact>(&conn, "SELECT id, email FROM users ORDER BY id")?;
  let Some(rejected) = decoded.pop() else {
    return Err("expected two rows".into());
  };
  let Some(normalized) = decoded.pop() else {
    return Err("expected two rows".into());
  };

  assert_eq!(
    normalized?,
    Contact {
      id: "u1".to_owned(),
      email: "ada@example.com".to_owned(),
    }
  );

  let msg = row_mapping_message(rejected)?;
  assert!(
    msg.contains("column 1 (email)") && msg.contains("not an email"),
    "message must name the column and carry the converter's error: {msg}"
  );
  Ok(())
}
