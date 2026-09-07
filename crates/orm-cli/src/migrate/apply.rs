//! Applying one migration, whatever supplied its bytes.
//!
//! The directory runner and the embedded runner differ only in where `sql`
//! comes from; everything from the hash check to the commit lives here so the
//! two cannot drift on transaction or rollback semantics.

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use super::error::MigrateError;
use super::store::record_migration;
use super::transaction::{begin, commit, rollback_after};

/// Checks `sql` against the hash declared for `name`.
pub(super) fn verify_hash(name: &str, sql: &str, expected: &str) -> Result<(), MigrateError> {
  let actual = compute_hash(sql);
  if actual == expected {
    return Ok(());
  }
  Err(MigrateError::HashMismatch {
    file: name.to_owned(),
    expected: expected.to_owned(),
    actual,
  })
}

/// Verifies the hash, then applies the statements and records the migration in
/// one transaction; any failure rolls the whole migration back.
pub(super) async fn apply_migration(
  conn: &impl DbConnection,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  verify_hash(name, sql, hash)?;

  begin(conn).await?;

  let result = async {
    execute_statements(conn, sql, name).await?;
    record_migration(conn, name, hash, dialect).await
  }
  .await;

  match result {
    Err(e) => Err(rollback_after(conn, e).await),
    Ok(()) => commit(conn).await,
  }
}

async fn execute_statements(
  conn: &impl DbConnection,
  content: &str,
  file_label: &str,
) -> Result<(), MigrateError> {
  for statement in content.split("--> statement-breakpoint") {
    if !has_statement(statement) {
      continue;
    }
    conn
      .execute_sql(statement.trim(), vec![])
      .await
      .map_err(|e| MigrateError::Database(format!("{file_label}: {e}")))?;
  }
  Ok(())
}

/// A chunk with only blank lines and `--` line comments (the generator emits
/// such chunks for operations a dialect cannot express, and never emits block
/// comments) must not reach the driver: libsql reports "not an error" when
/// asked to execute an empty statement.
fn has_statement(chunk: &str) -> bool {
  chunk
    .lines()
    .map(str::trim)
    .any(|line| !line.is_empty() && !line.starts_with("--"))
}
