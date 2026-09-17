//! Issue #86 for the blocking runners: `run_migrate_blocking` and
//! `run_migrate_embedded_blocking` validate an already-applied migration before
//! skipping it. In-memory rusqlite, no tokio runtime.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;
#[path = "fixtures/tampered_dir.rs"]
pub mod tampered_dir;

use toolu_orm_cli::migrate::{
  get_applied_migrations_blocking, run_migrate_blocking, run_migrate_embedded_blocking,
  MigrateError,
};
use toolu_orm_connection::RusqliteConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations};
use embedded_list::{honest, list, tampered};
use tampered_dir::{write_file, EDITED_LEDGER_SQL, LEDGER_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEDGER: &str = "0001_ledger.sql";

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  Ok(RusqliteConnection::from_connection(
    rusqlite::Connection::open_in_memory()?,
  ))
}

/// A database with the ledger migration applied from a journaled directory.
fn applied_ledger(dir: &str) -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  let conn = connect()?;
  write_migrations(dir, &[(LEDGER, LEDGER_SQL)])?;
  assert_eq!(run_migrate_blocking(&conn, dir, Dialect::Sqlite)?, 1);
  Ok(conn)
}

#[test]
fn editing_an_applied_file_fails_the_next_blocking_run() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir)?;
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;

  let Err(err) = run_migrate_blocking(&conn, &dir, Dialect::Sqlite) else {
    return Err("an edited applied migration was accepted".into());
  };
  let MigrateError::HashMismatch {
    file,
    expected,
    actual,
  } = &err
  else {
    return Err(format!("expected HashMismatch, got {err:?}").into());
  };
  assert_eq!(file, LEDGER);
  assert_eq!(expected, &compute_hash(LEDGER_SQL));
  assert_eq!(actual, &compute_hash(EDITED_LEDGER_SQL));
  assert_eq!(
    get_applied_migrations_blocking(&conn)?,
    vec![LEDGER.to_owned()]
  );
  Ok(())
}

#[test]
fn rewriting_an_applied_journal_hash_fails_the_next_blocking_run() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir)?;
  // File and journal rewritten together: only the database remembers what ran.
  write_migrations(&dir, &[(LEDGER, EDITED_LEDGER_SQL)])?;

  let Err(err) = run_migrate_blocking(&conn, &dir, Dialect::Sqlite) else {
    return Err("a rewritten journal entry was accepted".into());
  };
  let MigrateError::HistoryMismatch {
    file,
    recorded,
    declared,
  } = &err
  else {
    return Err(format!("expected HistoryMismatch, got {err:?}").into());
  };
  assert_eq!(file, LEDGER);
  assert_eq!(recorded, &compute_hash(LEDGER_SQL));
  assert_eq!(declared, &compute_hash(EDITED_LEDGER_SQL));
  Ok(())
}

#[test]
fn an_unchanged_directory_history_stays_idempotent_blocking() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir)?;

  assert_eq!(run_migrate_blocking(&conn, &dir, Dialect::Sqlite)?, 0);
  assert_eq!(
    get_applied_migrations_blocking(&conn)?,
    vec![LEDGER.to_owned()]
  );
  Ok(())
}

#[test]
fn an_edited_embedded_body_fails_the_next_blocking_run() -> TestResult {
  let conn = connect()?;
  let applied = vec![honest(LEDGER, LEDGER_SQL)];
  assert_eq!(
    run_migrate_embedded_blocking(&conn, &list(&applied), Dialect::Sqlite)?,
    1
  );

  let edited = vec![tampered(LEDGER, LEDGER_SQL, EDITED_LEDGER_SQL)];
  let Err(err) = run_migrate_embedded_blocking(&conn, &list(&edited), Dialect::Sqlite) else {
    return Err("an edited applied body was accepted".into());
  };
  assert!(matches!(err, MigrateError::HashMismatch { .. }), "{err:?}");
  Ok(())
}

#[test]
fn a_rehashed_embedded_entry_fails_the_next_blocking_run() -> TestResult {
  let conn = connect()?;
  let applied = vec![honest(LEDGER, LEDGER_SQL)];
  assert_eq!(
    run_migrate_embedded_blocking(&conn, &list(&applied), Dialect::Sqlite)?,
    1
  );

  let rehashed = vec![honest(LEDGER, EDITED_LEDGER_SQL)];
  let Err(err) = run_migrate_embedded_blocking(&conn, &list(&rehashed), Dialect::Sqlite) else {
    return Err("a rehashed applied entry was accepted".into());
  };
  let MigrateError::HistoryMismatch {
    file,
    recorded,
    declared,
  } = &err
  else {
    return Err(format!("expected HistoryMismatch, got {err:?}").into());
  };
  assert_eq!(file, LEDGER);
  assert_eq!(recorded, &compute_hash(LEDGER_SQL));
  assert_eq!(declared, &compute_hash(EDITED_LEDGER_SQL));
  Ok(())
}
