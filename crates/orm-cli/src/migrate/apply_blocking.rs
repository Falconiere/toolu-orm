//! Applying one migration over [`DbConnectionBlocking`].

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;

use super::apply::{has_statement, verify_hash};
use super::error::MigrateError;
use super::missing_extension::map_statement_error;
use super::store::record_migration_blocking;
use super::transaction_blocking::{begin, commit, rollback_after};

/// Verifies the hash, then applies the statements and records the migration in
/// one transaction; any failure rolls the whole migration back.
pub(super) fn apply_migration(
  conn: &impl DbConnectionBlocking,
  name: &str,
  sql: &str,
  hash: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  verify_hash(name, sql, hash)?;

  begin(conn)?;

  let result = (|| {
    execute_statements(conn, sql, name)?;
    record_migration_blocking(conn, name, hash, dialect)
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
    if !has_statement(statement) {
      continue;
    }
    conn
      .execute_sql(statement.trim(), vec![])
      .map_err(|e| map_statement_error(file_label, &e))?;
  }
  Ok(())
}
