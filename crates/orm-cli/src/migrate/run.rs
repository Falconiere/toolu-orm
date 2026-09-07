//! Migration runner that applies pending migrations from a directory in order.

use std::path::Path;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{Journal, JournalEntry};

use super::apply::apply_migration;
use super::error::MigrateError;
use super::pending::get_pending_migrations;
use super::store::{ensure_migrations_table, get_applied_migrations, record_migration};
use super::transaction::{begin, commit, rollback_after};

/// Applies pending migrations from the given directory to the database.
///
/// Migrations without a directory on the target machine — a single-binary
/// distribution — use [`run_migrate_embedded`](super::run_migrate_embedded)
/// instead; both share the same apply path.
///
/// # Errors
///
/// Returns `MigrateError` on database, I/O, or hash mismatch failures.
pub async fn run_migrate(
  conn: &impl DbConnection,
  migrations_dir: &str,
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  ensure_migrations_table(conn, dialect).await?;
  let applied = get_applied_migrations(conn).await?;

  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().unwrap_or("");

  let journal = Journal::read_from_path(journal_path_str)
    .map_err(|e| MigrateError::ReadFile(format!("{e}")))?;

  if journal.entries.is_empty() {
    let pending = get_pending_migrations(migrations_dir, &applied)?;
    let mut count: u32 = 0;
    for migration_file in &pending {
      apply_migration_legacy(conn, migrations_dir, migration_file, dialect).await?;
      count += 1;
    }
    return Ok(count);
  }

  let mut count: u32 = 0;
  for entry in &journal.entries {
    if applied.contains(&entry.name) {
      continue;
    }
    apply_journal_entry(conn, migrations_dir, entry, dialect).await?;
    count += 1;
  }

  Ok(count)
}

/// Reads the entry's file and hands it to the shared apply path.
async fn apply_journal_entry(
  conn: &impl DbConnection,
  migrations_dir: &str,
  entry: &JournalEntry,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let sql_path = Path::new(migrations_dir).join(&entry.name);
  let content = std::fs::read_to_string(&sql_path)
    .map_err(|e| MigrateError::ReadFile(format!("{}: {e}", sql_path.display())))?;

  apply_migration(conn, &entry.name, &content, &entry.hash, dialect).await
}

async fn apply_migration_legacy(
  conn: &impl DbConnection,
  migrations_dir: &str,
  migration_file: &str,
  dialect: Dialect,
) -> Result<(), MigrateError> {
  let path = Path::new(migrations_dir).join(migration_file);
  let sql = std::fs::read_to_string(&path)
    .map_err(|e| MigrateError::ReadFile(format!("{}: {e}", path.display())))?;

  begin(conn).await?;

  let result = async {
    conn
      .execute_batch(&sql)
      .await
      .map_err(|e| MigrateError::Database(format!("migration {migration_file}: {e}")))?;
    record_migration(conn, migration_file, "", dialect).await
  }
  .await;

  if let Err(e) = result {
    return Err(rollback_after(conn, e).await);
  }

  commit(conn).await
}
