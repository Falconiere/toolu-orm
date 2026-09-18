//! `VACUUM INTO`: the snapshot itself, its refusals, and validating it.

use toolu_orm_connection::{MaintenanceError, SqliteMaintenance};

use crate::temp_db_dir::{TempDbDir, TestResult, attached_schemas, rows_of, seeded_db};

/// The path reaches SQLite as a bound parameter, and what lands on disk is a
/// real database holding the same rows.
#[test]
fn vacuum_into_snapshots_a_real_database() -> TestResult {
  let dir = TempDbDir::new("snapshot")?;
  let source = seeded_db(&dir.file("src.db"), 3)?;
  let destination = dir.file("snap.db");

  source.vacuum_into(&destination)?;

  assert!(destination.is_file(), "the snapshot file was not created");
  let snapshot = rusqlite::Connection::open(&destination)?;
  assert_eq!(rows_of(&snapshot)?, vec!["row-0", "row-1", "row-2"]);
  assert_eq!(rows_of(&snapshot)?, rows_of(&source)?);
  Ok(())
}

/// SQLite will not overwrite a snapshot destination, and its refusal arrives
/// whole -- the variant, not just a rendered string -- with the existing
/// database untouched.
#[test]
fn vacuum_into_refuses_an_existing_destination() -> TestResult {
  let dir = TempDbDir::new("existing-dest")?;
  let source = seeded_db(&dir.file("src.db"), 1)?;
  let destination = dir.file("snap.db");
  // A real database that already holds different rows. A destination full of
  // junk would fail the header check first ("file is not a database"), which is
  // a different refusal from the one this test is about.
  let occupant = seeded_db(&destination, 3)?;
  let before = std::fs::read(&destination)?;

  let error = source
    .vacuum_into(&destination)
    .err()
    .ok_or("vacuuming onto an existing file must fail")?;

  assert!(
    matches!(
      &error,
      MaintenanceError::Sqlite(rusqlite::Error::SqliteFailure(failure, _))
        if failure.extended_code == rusqlite::ffi::SQLITE_ERROR
    ),
    "the driver error must survive with its result code, got {error:?}"
  );
  assert!(
    error.to_string().contains("output file already exists"),
    "SQLite's own message must survive, got {error}"
  );
  assert_eq!(
    rows_of(&occupant)?,
    vec!["row-0", "row-1", "row-2"],
    "the existing database must keep its own rows"
  );
  assert_eq!(
    std::fs::read(&destination)?,
    before,
    "the existing file must be left byte for byte as it was"
  );
  Ok(())
}

/// The validated-snapshot flow the issue asks for, end to end: vacuum, attach
/// the snapshot, check it, detach.
#[test]
fn a_vacuum_snapshot_passes_quick_check_through_an_attachment() -> TestResult {
  let dir = TempDbDir::new("validated")?;
  let source = seeded_db(&dir.file("src.db"), 5)?;
  let destination = dir.file("snap.db");

  source.vacuum_into(&destination)?;
  let snapshot = source.attach_database(&destination, "snapshot")?;
  let report = snapshot.quick_check()?;
  snapshot.detach()?;

  assert!(report.is_ok(), "a fresh snapshot must be healthy: {report}");
  assert_eq!(report.messages(), ["ok"]);
  assert_eq!(report.to_string(), "ok");
  assert_eq!(attached_schemas(&source)?, vec!["main"]);
  Ok(())
}

/// Both statements bind their filename, so a directory holding a single quote
/// and a double quote is not a special case for either of them.
#[test]
fn vacuum_into_and_attach_bind_a_path_containing_quotes() -> TestResult {
  let dir = TempDbDir::new("qu'ote\"dir")?;
  let dir_name = dir
    .path()
    .file_name()
    .and_then(std::ffi::OsStr::to_str)
    .ok_or("the temp directory must have a UTF-8 name")?;
  assert!(
    dir_name.ends_with("-qu'ote\"dir"),
    "the fixture must put both quote characters in the directory component, got {dir_name}"
  );
  let source_path = dir.file("sr'c\".db");
  assert_eq!(
    source_path.file_name().and_then(std::ffi::OsStr::to_str),
    Some("sr'c\".db"),
    "and in the file component"
  );
  let source = seeded_db(&source_path, 2)?;
  let destination = dir.file("sn'ap\".db");

  source.vacuum_into(&destination)?;

  let reader = rusqlite::Connection::open_in_memory()?;
  let snapshot = reader.attach_database(&destination, "snapshot")?;
  let rows: i64 = reader.query_row("SELECT count(*) FROM snapshot.t", [], |row| row.get(0))?;
  assert_eq!(rows, 2, "the attached snapshot must hold the source's rows");
  assert_eq!(snapshot.page_count()?, 2);
  Ok(())
}
