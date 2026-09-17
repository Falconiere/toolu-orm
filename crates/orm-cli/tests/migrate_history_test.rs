//! Issue #86: a migration already recorded in `_migrations` is validated before
//! it is skipped, by the async directory runner on in-memory libsql.
//!
//! Assertions go through raw libsql so the file compiles in every lane that has
//! libsql, whatever `FromRow` shape orm-core exposes.

#[path = "fixtures/baseline_dir.rs"]
pub mod baseline_dir;
#[path = "fixtures/tampered_dir.rs"]
pub mod tampered_dir;

use toolu_orm_cli::migrate::{mark_applied, run_migrate, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use baseline_dir::{migrations_dir, write_migrations};
use tampered_dir::{
  make_unreadable, prune_file, write_file, write_journal, AUDIT_SQL, EDITED_LEDGER_SQL, LEDGER_SQL,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEDGER: &str = "0001_ledger.sql";
const AUDIT: &str = "0002_audit.sql";

async fn connect() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  Ok(Database::init_local(":memory:").await?.connect()?)
}

async fn scalar(conn: &LibsqlConnection, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn.inner_conn().query(sql, ()).await?;
  let row = rows.next().await?.ok_or("scalar query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

async fn recorded(conn: &LibsqlConnection) -> Result<i64, Box<dyn std::error::Error>> {
  scalar(conn, "SELECT count(*) FROM _migrations").await
}

async fn has_table(conn: &LibsqlConnection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
  let mut rows = conn
    .inner_conn()
    .query(
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
      libsql::params![name],
    )
    .await?;
  let row = rows.next().await?.ok_or("table query returned no row")?;
  Ok(row.get::<i64>(0)?)
}

/// A database with `0001_ledger.sql` applied from a journaled directory — the
/// starting point every tampering test edits from.
async fn applied_ledger(dir: &str) -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  write_migrations(dir, &[(LEDGER, LEDGER_SQL)])?;
  assert_eq!(run_migrate(&conn, dir, Dialect::Sqlite).await?, 1);
  Ok(conn)
}

#[tokio::test]
async fn editing_an_applied_file_fails_the_next_run() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let conn = applied_ledger(&dir).await?;
  write_file(&dir, LEDGER, EDITED_LEDGER_SQL)?;

  let Err(err) = run_migrate(&conn, &dir, Dialect::Sqlite).await else {
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
  assert!(matches!(err, MigrateError::HashMismatch { .. }), "{err:?}");
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
  assert!(
    matches!(err, MigrateError::HistoryMismatch { .. }),
    "{err:?}"
  );
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
  assert!(matches!(err, MigrateError::HashMismatch { .. }), "{err:?}");
  assert_eq!(
    has_table(&conn, "audit").await?,
    0,
    "0002 ran despite the tampered history"
  );
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
