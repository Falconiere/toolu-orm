//! Applying one migration, whatever supplied its bytes.
//!
//! The directory runner and the embedded runner differ only in where `sql`
//! comes from; everything from the hash check to the commit lives here so the
//! two cannot drift on transaction or rollback semantics.

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::compute_hash;

use super::error::MigrateError;
use super::missing_extension::map_statement_error;
use super::pragma_guard::{
  arm_pragmas, check_foreign_keys, is_foreign_keys_pragma, restore_pragmas, PragmaGuard,
};
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
///
/// A SQLite migration that asks for `PRAGMA foreign_keys = OFF` gets it around
/// the transaction rather than inside it, where SQLite documents the pragma as
/// a no-op; the original setting comes back on every exit path.
pub(super) async fn apply_migration(
  conn: &impl DbConnection,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  verify_hash(name, sql, hash)?;

  let guard = arm_pragmas(conn, sql, dialect).await?;
  let outcome = apply_in_transaction(conn, name, sql, hash, dialect, guard).await;
  restore_pragmas(conn, guard, outcome).await
}

/// The transaction itself: statements, the `_migrations` row, then the
/// referential-integrity check that must pass before the commit.
async fn apply_in_transaction(
  conn: &impl DbConnection,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
  guard: PragmaGuard,
) -> Result<(), MigrateError> {
  begin(conn).await?;

  let result = async {
    execute_statements(conn, sql, name).await?;
    record_migration(conn, name, hash, dialect).await?;
    check_foreign_keys(conn, guard, name).await
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
    if !has_statement(statement) || is_foreign_keys_pragma(statement) {
      continue;
    }
    // `execute_batch`: a chunk may hold several `;`-separated statements (plain
    // SQL files). `execute_sql` is single-statement on rusqlite (`MultipleStatement`).
    conn
      .execute_batch(statement.trim())
      .await
      .map_err(|e| map_statement_error(file_label, &e))?;
  }
  Ok(())
}

/// A chunk with only blank lines and `--` line comments (the generator emits
/// such chunks for operations a dialect cannot express, and never emits block
/// comments) must not reach the driver: libsql reports "not an error" when
/// asked to execute an empty statement.
pub(super) fn has_statement(chunk: &str) -> bool {
  chunk
    .lines()
    .map(str::trim)
    .any(|line| !line.is_empty() && !line.starts_with("--"))
}
