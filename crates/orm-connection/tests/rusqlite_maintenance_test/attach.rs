//! `ATTACH DATABASE`: quoting, cleanup on every exit path, and refusals.

use std::path::Path;

use toolu_orm_connection::{MaintenanceError, SqliteMaintenance};

use crate::temp_db_dir::{TempDbDir, TestResult, attached_schemas, seeded_db};

/// A schema name holding a double quote is rendered correctly, addresses the
/// database it names, and is reported back unchanged -- and the guard removes it
/// when it goes out of scope.
#[test]
fn attach_quotes_a_schema_name_containing_a_quote() -> TestResult {
  let dir = TempDbDir::new("quoted-schema")?;
  let path = dir.file("other.db");
  drop(seeded_db(&path, 2)?);

  // An empty main database, so a row count can only come from the attachment.
  let conn = rusqlite::Connection::open_in_memory()?;
  {
    let attached = conn.attach_database(&path, "we\"ird")?;

    assert_eq!(attached.schema(), "we\"ird");
    assert_eq!(
      attached_schemas(&conn)?,
      vec!["main".to_owned(), "we\"ird".to_owned()]
    );
    let rows: i64 = conn.query_row("SELECT count(*) FROM \"we\"\"ird\".t", [], |row| row.get(0))?;
    assert_eq!(rows, 2, "the quoted schema must address the attached file");
    assert!(attached.quick_check()?.is_ok());
    assert_eq!(attached.page_size()?, 4096);
  }

  assert_eq!(
    attached_schemas(&conn)?,
    vec!["main"],
    "the guard must detach when it goes out of scope"
  );
  Ok(())
}

/// Attach, then fail. The `?` abandons the copy and the attachment goes with it,
/// which is the whole reason the guard exists.
#[test]
fn a_failed_copy_still_detaches_the_attached_database() -> TestResult {
  let dir = TempDbDir::new("failed-copy")?;
  let source = dir.file("old.db");
  drop(seeded_db(&source, 2)?);
  let conn = seeded_db(&dir.file("new.db"), 0)?;

  let error = copy_from_attached(&conn, &source).expect_err("the copy must fail");

  assert!(
    error.to_string().contains("no such table"),
    "SQLite's own message must survive the early return, got {error}"
  );
  assert_eq!(
    attached_schemas(&conn)?,
    vec!["main"],
    "the failed copy must leave nothing attached"
  );
  // The proof that the detach really happened: the same name is free again.
  let retry = conn.attach_database(&source, "old")?;
  assert_eq!(retry.schema(), "old");
  Ok(())
}

/// Attaches, then runs a copy that cannot work. Every exit is through `?`, so
/// nothing here detaches by hand.
fn copy_from_attached(conn: &rusqlite::Connection, source: &Path) -> Result<(), MaintenanceError> {
  let _old = conn.attach_database(source, "old")?;
  conn.execute("INSERT INTO t (a) SELECT a FROM old.does_not_exist", [])?;
  Ok(())
}

/// `detach()` clears the attachment and reports its own failure instead of
/// swallowing it the way `Drop` has to.
#[test]
fn explicit_detach_clears_the_attachment_and_reports_its_own_failure() -> TestResult {
  let dir = TempDbDir::new("explicit-detach")?;
  let path = dir.file("old.db");
  drop(seeded_db(&path, 1)?);
  let conn = rusqlite::Connection::open_in_memory()?;

  conn.attach_database(&path, "old")?.detach()?;
  assert_eq!(attached_schemas(&conn)?, vec!["main"]);
  // Free again, so the detach was real and happened exactly where it was asked.
  conn.attach_database(&path, "old")?.detach()?;

  let stolen = conn.attach_database(&path, "old")?;
  conn.execute_batch("DETACH DATABASE \"old\"")?;
  let error = stolen
    .detach()
    .expect_err("detaching a schema that is already gone must fail");

  assert!(
    matches!(
      error,
      MaintenanceError::Sqlite(rusqlite::Error::SqliteFailure(..))
    ),
    "the driver error must survive as itself, got {error:?}"
  );
  assert!(
    error.to_string().contains("no such database"),
    "SQLite's own message must survive, got {error}"
  );
  Ok(())
}

/// Two identifiers this API refuses itself, before any statement is built.
#[test]
fn attach_rejects_an_unusable_schema_name() -> TestResult {
  let dir = TempDbDir::new("bad-names")?;
  let path = dir.file("old.db");
  drop(seeded_db(&path, 1)?);
  let conn = rusqlite::Connection::open_in_memory()?;

  let empty = conn
    .attach_database(&path, "")
    .err()
    .ok_or("an empty schema name must be refused")?;
  assert!(
    matches!(
      &empty,
      MaintenanceError::InvalidSchemaName { name, reason }
        if name.is_empty() && *reason == "it is empty"
    ),
    "got {empty:?}"
  );

  let nul = conn
    .attach_database(&path, "a\0b")
    .err()
    .ok_or("a schema name holding a NUL byte must be refused")?;
  assert!(
    matches!(
      &nul,
      MaintenanceError::InvalidSchemaName { reason, .. }
        if *reason == "it contains a NUL byte"
    ),
    "got {nul:?}"
  );

  assert_eq!(
    attached_schemas(&conn)?,
    vec!["main"],
    "a refused name must not reach the driver"
  );
  Ok(())
}

/// SQLite's own refusals are handed back whole, on the attach path too.
#[test]
fn attaching_a_file_that_is_not_a_database_preserves_the_sqlite_error() -> TestResult {
  let dir = TempDbDir::new("not-a-db")?;
  let junk = dir.file("junk.db");
  std::fs::write(&junk, b"plainly not a SQLite database, no header at all")?;
  let conn = rusqlite::Connection::open_in_memory()?;

  let error = conn
    .attach_database(&junk, "junk")
    .err()
    .ok_or("attaching a non-database must fail")?;

  assert!(
    matches!(
      error,
      MaintenanceError::Sqlite(rusqlite::Error::SqliteFailure(..))
    ),
    "the driver error must survive as itself, got {error:?}"
  );
  assert!(
    error.to_string().contains("file is not a database"),
    "SQLite's own message must survive, got {error}"
  );
  assert_eq!(attached_schemas(&conn)?, vec!["main"]);
  Ok(())
}
