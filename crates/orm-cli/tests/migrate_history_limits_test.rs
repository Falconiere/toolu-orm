//! Issue #86: the documented limits of the applied-history check, and the
//! idempotence it must not cost. In-memory libsql, async directory runner.
//!
//! A row recorded without a hash cannot be verified, a journal that no longer
//! lists a name declares nothing about it, and a pruned `.sql` file leaves the
//! recorded-against-declared comparison as the whole verdict — while a file
//! that exists and cannot be read is still an error.
//!
//! The tamper cases live in `migrate_history_test.rs`.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/libsql_probe.rs"]
pub mod libsql_probe;
#[path = "fixtures/tampered_dir.rs"]
pub mod tampered_dir;

use toolu_orm_cli::migrate::{run_migrate, MigrateError};
use toolu_orm_connection::LibsqlConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations};
use libsql_probe::{connect, has_table, scalar};
use tampered_dir::{
  make_unreadable, prune_file, write_file, write_journal, AUDIT_SQL, EDITED_LEDGER_SQL, LEDGER_SQL,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEDGER: &str = "0001_ledger.sql";
const AUDIT: &str = "0002_audit.sql";

async fn recorded(conn: &LibsqlConnection) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(conn, "SELECT count(*) FROM _migrations").await
}

/// A database with `0001_ledger.sql` applied from a journaled directory.
async fn applied_ledger(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  write_migrations(dir, &[(LEDGER, LEDGER_SQL)])?;
  assert_eq!(run_migrate(&conn, dir, Dialect::Sqlite).await?, 1);
  Ok(conn)
}

#[tokio::test]
async fn a_row_recorded_without_a_hash_is_skipped_unverified() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  // No `_journal.json`: the journal-free path records an empty hash.
  write_file(&dir, LEDGER, LEDGER_SQL)?;
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 1);
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM _migrations WHERE hash = ''").await?,
    1
  );

  // The project adopts a journal later, and the file has changed since. There
  // is no recorded hash to compare, so the entry is skipped, not verified.
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;
  write_journal(&dir, &[(LEDGER, &compute_hash(EDITED_LEDGER_SQL))])?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 0);
  assert_eq!(recorded(&conn).await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_journal_entry_dropped_from_a_pruned_history_is_not_validated() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  write_migrations(&dir, &[(LEDGER, LEDGER_SQL), (AUDIT, AUDIT_SQL)])?;
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 2);

  // The history is squashed to its tail, and the dropped file changes. The
  // journal declares nothing about `0001`, so there is nothing to compare.
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;
  write_journal(&dir, &[(AUDIT, &compute_hash(AUDIT_SQL))])?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 0);
  assert_eq!(recorded(&conn).await?, 2);
  Ok(())
}

#[tokio::test]
async fn an_applied_file_pruned_from_disk_passes_on_the_recorded_hash() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  prune_file(&dir, LEDGER)?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 0);
  assert_eq!(recorded(&conn).await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_pruned_file_whose_journal_hash_changed_still_fails() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  prune_file(&dir, LEDGER)?;
  write_journal(&dir, &[(LEDGER, &compute_hash(EDITED_LEDGER_SQL))])?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("a pruned file excused a rewritten journal hash".into());
  };
  let MigrateError::HistoryMismatch {
    file,
    recorded: was,
    declared,
  } = &err
  else {
    return Err(format!("expected HistoryMismatch, got {err:?}").into());
  };
  assert_eq!(file, LEDGER);
  assert_eq!(was, &compute_hash(LEDGER_SQL));
  assert_eq!(declared, &compute_hash(EDITED_LEDGER_SQL));
  Ok(())
}

#[tokio::test]
async fn an_unreadable_applied_file_is_a_read_file_error() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  make_unreadable(&dir, LEDGER)?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("an unreadable applied file passed for a pruned one".into());
  };
  assert!(matches!(err, MigrateError::ReadFile(_)), "{err:?}");
  assert!(err.to_string().contains(LEDGER), "message: {err}");
  Ok(())
}

#[tokio::test]
async fn an_unchanged_history_stays_idempotent() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  write_migrations(&dir, &[(LEDGER, LEDGER_SQL), (AUDIT, AUDIT_SQL)])?;

  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 2);
  assert_eq!(run_migrate(&conn, &dir, Dialect::Sqlite).await?, 0);
  assert_eq!(recorded(&conn).await?, 2);
  assert_eq!(has_table(&conn, "audit").await?, 1);
  Ok(())
}
