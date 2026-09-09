//! Blocking directory migration runner over [`DbConnectionBlocking`].

use std::path::Path;

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{Journal, JournalEntry};

use super::apply_blocking::apply_migration;
use super::error::MigrateError;
use super::pending::get_pending_migrations;
use super::store::{
  ensure_migrations_table_blocking, get_applied_migrations_blocking, record_migration_blocking,
};
use super::transaction_blocking::{begin, commit, rollback_after};

/// Blocking twin of [`super::run_migrate`].
///
/// # Errors
///
/// Returns `MigrateError` on database, I/O, or hash mismatch failures.
pub fn run_migrate_blocking(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  ensure_migrations_table_blocking(conn, dialect)?;
  let applied = get_applied_migrations_blocking(conn)?;

  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().unwrap_or("");

  let journal = Journal::read_from_path(journal_path_str)
    .map_err(|e| MigrateError::ReadFile(format!("{e}")))?;

  if journal.entries.is_empty() {
    let pending = get_pending_migrations(migrations_dir, &applied)?;
    let mut count: u32 = 0;
    for migration_file in &pending {
      apply_migration_legacy(conn, migrations_dir, migration_file, dialect)?;
      count += 1;
    }
    return Ok(count);
  }

  let mut count: u32 = 0;
  for entry in &journal.entries {
    if applied.contains(&entry.name) {
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

  begin(conn)?;

  let result = (|| {
    conn
      .execute_batch(&sql)
      .map_err(|e| MigrateError::Database(format!("migration {migration_file}: {e}")))?;
    record_migration_blocking(conn, migration_file, "", dialect)
  })();

  if let Err(e) = result {
    return Err(rollback_after(conn, e));
  }

  commit(conn)
}
