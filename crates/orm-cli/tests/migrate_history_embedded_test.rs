//! Issue #86 for the embedded runner: the body and the declared hash of an
//! already-applied name are validated before that name is skipped. Async, on
//! in-memory libsql.

#[path = "fixtures/embedded_list.rs"]
pub mod embedded_list;
#[path = "fixtures/tampered_dir.rs"]
pub mod tampered_dir;

use toolu_orm_cli::migrate::{run_migrate_embedded, MigrateError};
use toolu_orm_connection::{Database, LibsqlConnection};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use embedded_list::{honest, list, tampered, OwnedMigration};
use tampered_dir::{AUDIT_SQL, EDITED_LEDGER_SQL, LEDGER_SQL};

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

/// A database with the ledger migration applied from an embedded list.
async fn applied_ledger() -> Result<LibsqlConnection, Box<dyn std::error::Error>> {
  let conn = connect().await?;
  let applied = vec![honest(LEDGER, LEDGER_SQL)];
  assert_eq!(
    run_migrate_embedded(&conn, &list(&applied), Dialect::Sqlite).await?,
    1
  );
  Ok(conn)
}

#[tokio::test]
async fn an_edited_body_for_an_applied_name_fails() -> TestResult {
  let conn = applied_ledger().await?;
  // Same name, same declared hash, different SQL: the shipped file was edited.
  let edited = vec![tampered(LEDGER, LEDGER_SQL, EDITED_LEDGER_SQL)];

  let Err(err) = run_migrate_embedded(&conn, &list(&edited), Dialect::Sqlite).await else {
    return Err("an edited applied body was accepted".into());
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
  Ok(())
}

#[tokio::test]
async fn a_rehashed_entry_for_an_applied_name_is_a_history_mismatch() -> TestResult {
  let conn = applied_ledger().await?;
  // Body and declared hash were changed together, so only the database
  // remembers what actually ran.
  let rehashed = vec![honest(LEDGER, EDITED_LEDGER_SQL)];

  let Err(err) = run_migrate_embedded(&conn, &list(&rehashed), Dialect::Sqlite).await else {
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

#[tokio::test]
async fn a_tampered_entry_blocks_a_pending_embedded_entry() -> TestResult {
  let conn = applied_ledger().await?;
  let next: Vec<OwnedMigration> = vec![
    tampered(LEDGER, LEDGER_SQL, EDITED_LEDGER_SQL),
    honest(AUDIT, AUDIT_SQL),
  ];

  let Err(err) = run_migrate_embedded(&conn, &list(&next), Dialect::Sqlite).await else {
    return Err("a tampered history let a pending entry through".into());
  };
  assert!(matches!(err, MigrateError::HashMismatch { .. }), "{err:?}");
  assert_eq!(
    scalar(&conn, "SELECT count(*) FROM _migrations").await?,
    1,
    "a row was recorded despite the tampered history"
  );
  assert_eq!(
    scalar(
      &conn,
      "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'audit'"
    )
    .await?,
    0,
    "the pending entry ran despite the tampered history"
  );
  Ok(())
}

#[tokio::test]
async fn an_unchanged_embedded_list_stays_idempotent() -> TestResult {
  let conn = connect().await?;
  let migrations = vec![honest(LEDGER, LEDGER_SQL), honest(AUDIT, AUDIT_SQL)];

  assert_eq!(
    run_migrate_embedded(&conn, &list(&migrations), Dialect::Sqlite).await?,
    2
  );
  assert_eq!(
    run_migrate_embedded(&conn, &list(&migrations), Dialect::Sqlite).await?,
    0
  );
  assert_eq!(scalar(&conn, "SELECT count(*) FROM _migrations").await?, 2);
  Ok(())
}
