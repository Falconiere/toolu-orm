//! Issue #86: the async directory runner rejects a change to a migration it has
//! already applied, before it runs anything pending. In-memory libsql.
//!
//! The limits of that check — unhashed rows, pruned histories, unreadable files
//! — live in `migrate_history_limits_test.rs`.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/libsql_probe.rs"]
pub mod libsql_probe;
#[path = "fixtures/tampered_dir.rs"]
pub mod tampered_dir;

use toolu_orm_cli::migrate::{mark_applied, run_migrate, MigrateError};
use toolu_orm_connection::LibsqlConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations};
use libsql_probe::{connect, has_table, scalar};
use tampered_dir::{write_file, write_journal, AUDIT_SQL, EDITED_LEDGER_SQL, LEDGER_SQL};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEDGER: &str = "0001_ledger.sql";
const AUDIT: &str = "0002_audit.sql";

async fn recorded(conn: &LibsqlConnection) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(conn, "SELECT count(*) FROM _migrations").await
}

/// A database with `0001_ledger.sql` applied from a journaled directory — the
/// starting point every tampering test edits from.
async fn applied_ledger(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  write_migrations(dir, &[(LEDGER, LEDGER_SQL)])?;
  assert_eq!(run_migrate(&conn, dir, Dialect::Sqlite).await?, 1);
  Ok(conn)
}

/// The `HashMismatch` an edited file must produce, with both hashes checked.
fn assert_edited_ledger(err: &MigrateError) -> TestResult {
  let MigrateError::HashMismatch {
    file,
    expected,
    actual,
  } = err
  else {
    return Err(format!("expected HashMismatch, got {err:?}").into());
  };
  assert_eq!(file, LEDGER);
  assert_eq!(expected, &compute_hash(LEDGER_SQL));
  assert_eq!(actual, &compute_hash(EDITED_LEDGER_SQL));
  Ok(())
}

#[tokio::test]
async fn editing_an_applied_file_fails_the_next_run() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("an edited applied migration was accepted".into());
  };
  assert_edited_ledger(&err)?;
  assert_eq!(recorded(&conn).await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_tampered_entry_blocks_a_pending_migration() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;
  write_file(&dir, AUDIT, AUDIT_SQL)?;
  write_journal(
    &dir,
    &[
      (LEDGER, &compute_hash(LEDGER_SQL)),
      (AUDIT, &compute_hash(AUDIT_SQL)),
    ],
  )?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("a tampered history let a pending migration through".into());
  };
  assert_edited_ledger(&err)?;
  assert_eq!(
    recorded(&conn).await?,
    1,
    "a row was recorded despite the tampered history"
  );
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "0002 ran despite the tampered history"
  );
  Ok(())
}

#[tokio::test]
async fn rewriting_an_applied_journal_hash_is_a_history_mismatch() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  // File and journal are rewritten together, so only the database remembers
  // what was actually applied.
  write_migrations(&dir, &[(LEDGER, EDITED_LEDGER_SQL)])?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("a rewritten journal entry was accepted".into());
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
  assert_eq!(recorded(&conn).await?, 1);
  Ok(())
}

#[tokio::test]
async fn a_baselined_migration_is_still_tamper_evident() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = connect().await?;
  write_migrations(&dir, &[(LEDGER, LEDGER_SQL), (AUDIT, AUDIT_SQL)])?;
  // Adoption: 0001 is recorded from the journal without ever being executed,
  // so the hash it carries is the journal's.
  assert_eq!(
    mark_applied(&conn, &dir, &[LEDGER], Dialect::Sqlite).await?,
    1
  );
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
    return Err("an edited baselined migration was accepted".into());
  };
  assert_edited_ledger(&err)?;
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "0002 ran despite the tampered history"
  );
  Ok(())
}
