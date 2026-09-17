//! Applying one migration over [`DbConnectionBlocking`].

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;

use super::apply::{has_statement, verify_hash};
use super::error::MigrateError;
use super::missing_extension::map_statement_error;
use super::pragma_guard::{is_foreign_keys_pragma, PragmaGuard};
use super::pragma_guard_blocking::{arm_pragmas, check_foreign_keys, restore_pragmas};
use super::store::record_migration_blocking;
use super::transaction_blocking::{begin, commit, rollback_after};

/// Verifies the hash, then applies the statements and records the migration in
/// one transaction; any failure rolls the whole migration back.
///
/// Blocking twin of [`super::apply::apply_migration`], including its handling
/// of a `PRAGMA foreign_keys` the migration asks for.
pub(super) fn apply_migration(
  conn: &impl DbConnectionBlocking,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  verify_hash(name, sql, hash)?;

  let guard = arm_pragmas(conn, sql, dialect)?;
  let outcome = apply_in_transaction(conn, name, sql, hash, dialect, guard);
  restore_pragmas(conn, guard, outcome)
}

/// The transaction itself: statements, the `_migrations` row, then the
/// referential-integrity check that must pass before the commit.
fn apply_in_transaction(
  conn: &impl DbConnectionBlocking,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
  guard: PragmaGuard,
) -> Result<(), MigrateError> {
  begin(conn)?;

  let result = (|| {
    execute_statements(conn, sql, name)?;
    record_migration_blocking(conn, name, hash, dialect)?;
    check_foreign_keys(conn, guard, name)
  })();

  match result {
    Err(e) => Err(rollback_after(conn, e)),
    Ok(()) => commit(conn),
  }
}

fn execute_statements(
  conn: &impl DbConnectionBlocking,
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
      .map_err(|e| map_statement_error(file_label, &e))?;
  }
  Ok(())
}
