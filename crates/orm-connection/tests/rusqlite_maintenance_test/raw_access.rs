//! Paths this API refuses outright, and the escape hatch out of the wrapper.

use toolu_orm_connection::{
  DbConnectionBlocking, MaintenanceError, RusqliteConnection, SqliteMaintenance,
};
use toolu_orm_core::value::Value;

use crate::temp_db_dir::{TempDbDir, TestResult, attached_schemas, seeded_db};

/// SQLite takes filenames as UTF-8 text, so a path that is not UTF-8 is refused
/// here rather than lossily converted -- and nothing reaches the driver.
#[cfg(unix)]
#[test]
fn a_non_utf8_path_is_refused_before_the_driver() -> TestResult {
  use std::os::unix::ffi::OsStrExt;

  let dir = TempDbDir::new("non-utf8")?;
  let bad = dir
    .path()
    .join(std::ffi::OsStr::from_bytes(b"\xff\xfe-not-utf8.db"));
  let conn = rusqlite::Connection::open_in_memory()?;

  let vacuum = conn
    .vacuum_into(&bad)
    .err()
    .ok_or("a non-UTF-8 destination must be refused")?;
  assert!(
    matches!(vacuum, MaintenanceError::NonUtf8Path(_)),
    "got {vacuum:?}"
  );

  let attach = conn
    .attach_database(&bad, "bad")
    .err()
    .ok_or("a non-UTF-8 attach path must be refused")?;
  assert!(
    matches!(attach, MaintenanceError::NonUtf8Path(_)),
    "got {attach:?}"
  );

  assert!(!bad.exists(), "no file may be created for a refused path");
  assert_eq!(attached_schemas(&conn)?, vec!["main"]);
  Ok(())
}

/// `with_raw_connection` is the documented way back to the driver after
/// `from_connection` took ownership: the maintenance surface works through it,
/// and what the closure does is the wrapper's own database.
#[test]
fn with_raw_connection_borrows_the_driver_connection() -> TestResult {
  let dir = TempDbDir::new("wrapper")?;
  let path = dir.file("wrapped.db");
  let raw = seeded_db(&path, 2)?;
  let direct = raw.storage_stats()?;

  let conn = RusqliteConnection::from_connection(raw);
  let through = conn.with_raw_connection(SqliteMaintenance::storage_stats)??;
  assert_eq!(through, direct, "the closure must see the wrapped database");
  assert!(
    conn
      .with_raw_connection(SqliteMaintenance::quick_check)??
      .is_ok()
  );

  // A statement issued inside the closure is the wrapper's own state...
  conn
    .with_raw_connection(|driver| driver.execute_batch("CREATE TABLE made_inside (x TEXT)"))??;
  let affected = DbConnectionBlocking::execute_sql(
    &conn,
    "INSERT INTO made_inside (x) VALUES (?1)",
    vec![Value::Text("through the wrapper".to_owned())],
  )?;
  assert_eq!(affected, 1);

  // ...and the wrapper's writes are what the closure reads back.
  let rows = conn.with_raw_connection(|driver| {
    driver.query_row("SELECT count(*) FROM made_inside", [], |row| {
      row.get::<_, i64>(0)
    })
  })??;
  assert_eq!(rows, 1);
  Ok(())
}
