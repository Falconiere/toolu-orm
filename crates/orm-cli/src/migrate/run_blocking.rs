//! Blocking directory migration runner over [`DbConnectionBlocking`].

use std::path::Path;

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{Journal, JournalEntry};

use super::apply_blocking::apply_migration;
use super::error::MigrateError;
use super::history::validate_directory_history;
use super::missing_extension::map_statement_error;
use super::pending::get_pending_migrations;
use super::pragma_guard::PragmaGuard;
use super::pragma_guard_blocking::{arm_pragmas, check_foreign_keys, restore_pragmas};
use super::store::{
  ensure_migrations_table_blocking, get_applied_history_blocking, record_migration_blocking,
};
use super::transaction_blocking::{begin, commit, rollback_after};

/// Blocking twin of [`super::run_migrate`], including its validation of the
/// history already in `_migrations`.
///
/// # Errors
///
/// Same as [`super::run_migrate`]: [`MigrateError::HistoryMismatch`],
/// [`MigrateError::HashMismatch`], or a database or I/O failure.
pub fn run_migrate_blocking(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  ensure_migrations_table_blocking(conn, dialect)?;
  let applied = get_applied_history_blocking(conn)?;

  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().ok_or_else(|| {
    MigrateError::ReadFile(format!("{} is not valid UTF-8", journal_path.display()))
  })?;

  let journal = Journal::read_from_path(journal_path_str)
    .map_err(|e| MigrateError::ReadFile(format!("{e}")))?;

  if journal.entries.is_empty() {
    // This branch returns, so consuming `applied` here costs the journaled path
    // below nothing: it still owns the records it validates against.
    let names: Vec<String> = applied.into_iter().map(|record| record.name).collect();
    let pending = get_pending_migrations(migrations_dir, &names)?;
    let mut count: u32 = 0;
    for migration_file in &pending {
      apply_migration_legacy(conn, migrations_dir, migration_file, dialect)?;
      count += 1;
    }
    return Ok(count);
  }

  validate_directory_history(migrations_dir, &journal.entries, &applied)?;

  let mut count: u32 = 0;
  for entry in &journal.entries {
    if applied.iter().any(|record| record.name == entry.name) {
      continue;
    }
    apply_journal_entry(conn, migrations_dir, entry, dialect)?;
    count += 1;
  }

  Ok(count)
}

fn apply_journal_entry(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  entry: &JournalEntry,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let sql_path = Path::new(migrations_dir).join(&entry.name);
  let content = std::fs::read_to_string(&sql_path)
    .map_err(|e| MigrateError::ReadFile(format!("{}: {e}", sql_path.display())))?;

  apply_migration(conn, &entry.name, &content, &entry.hash, dialect)
}

fn apply_migration_legacy(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  migration_file: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let path = Path::new(migrations_dir).join(migration_file);
  let sql = std::fs::read_to_string(&path)
    .map_err(|e| MigrateError::ReadFile(format!("{}: {e}", path.display())))?;

  let guard = arm_pragmas(conn, &sql, dialect)?;
  let outcome = legacy_in_transaction(conn, &sql, migration_file, dialect, guard);
  restore_pragmas(conn, guard, outcome)
}

/// A plain (journal-free) file is executed as one batch, so its statements are
/// not split; the foreign-key handling is the same as for a journal entry.
fn legacy_in_transaction(
  conn: &impl DbConnectionBlocking,
  sql: &str,
  migration_file: &str,
  dialect: Dialect,
  guard: PragmaGuard,
) -> Result<(), MigrateError> {
  begin(conn)?;

  let result = (|| {
    conn
      .execute_batch(sql)
      .map_err(|e| map_statement_error(migration_file, &e))?;
    record_migration_blocking(conn, migration_file, "", dialect)?;
    check_foreign_keys(conn, guard, migration_file)
  })();

  if let Err(e) = result {
    return Err(rollback_after(conn, e));
  }

  commit(conn)
}
