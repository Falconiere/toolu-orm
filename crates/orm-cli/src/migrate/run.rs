//! Migration runner that applies pending migrations in order.

use std::path::Path;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::{compute_hash, Journal};

use super::error::MigrateError;
use super::pending::get_pending_migrations;
use super::store::{ensure_migrations_table, get_applied_migrations, record_migration};

/// Applies pending migrations from the given directory to the database.
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

    let sql_path = Path::new(migrations_dir).join(&entry.name);
    let content = std::fs::read_to_string(&sql_path)
      .map_err(|e| MigrateError::ReadFile(format!("{}: {e}", sql_path.display())))?;

    let actual_hash = compute_hash(&content);
    if actual_hash != entry.hash {
      return Err(MigrateError::HashMismatch {
        file: entry.name.clone(),
        expected: entry.hash.clone(),
        actual: actual_hash,
      });
    }

    conn
      .execute_batch("BEGIN")
      .await
      .map_err(|e| MigrateError::Database(format!("begin transaction: {e}")))?;

    let exec_result = async {
      execute_migration_statements(conn, &content, &entry.name).await?;
      record_migration(conn, &entry.name, &entry.hash, dialect).await
    }
    .await;

    match exec_result {
      Err(e) => {
        let _ = conn.execute_batch("ROLLBACK").await;
        return Err(e);
      },
      Ok(()) => {
        conn
          .execute_batch("COMMIT")
          .await
          .map_err(|e| MigrateError::Database(format!("commit transaction: {e}")))?;
      },
    }

    count += 1;
  }

  Ok(count)
}

async fn execute_migration_statements(
  conn: &impl DbConnection,
  content: &str,
  file_label: &str,
) -> Result<(), MigrateError> {
  for statement in content.split("--> statement-breakpoint") {
    let trimmed = statement.trim();
    if trimmed.is_empty() {
      continue;
    }
    conn
      .execute_sql(trimmed, vec![])
      .await
      .map_err(|e| MigrateError::Database(format!("{file_label}: {e}")))?;
  }
  Ok(())
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

  conn
    .execute_batch("BEGIN")
    .await
    .map_err(|e| MigrateError::Database(format!("begin transaction: {e}")))?;

  let result = async {
    conn
      .execute_batch(&sql)
      .await
      .map_err(|e| MigrateError::Database(format!("migration {migration_file}: {e}")))?;
    record_migration(conn, migration_file, "", dialect).await
  }
  .await;

  if result.is_err() {
    let _ = conn.execute_batch("ROLLBACK").await;
    return result;
  }

  conn
    .execute_batch("COMMIT")
    .await
    .map_err(|e| MigrateError::Database(format!("commit transaction: {e}")))?;

  result
}
