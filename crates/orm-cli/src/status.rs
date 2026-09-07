//! Migration status reporting (applied vs pending).

use std::path::Path;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;

use crate::migrate::{ensure_migrations_table, get_applied_migrations, MigrateError};

pub struct MigrationStatus {
  pub applied: Vec<String>,
  pub pending: Vec<String>,
}

/// Returns the current migration status.
///
/// # Errors
///
/// Returns `MigrateError` on database or file I/O failures.
pub async fn get_status(
  conn: &impl DbConnection,
  migrations_dir: &str,
  dialect: Dialect,
) -> Result<MigrationStatus, MigrateError> {
  ensure_migrations_table(conn, dialect).await?;
  let applied = get_applied_migrations(conn).await?;
  let all_files = collect_all_sql_files(migrations_dir)?;

  let pending: Vec<String> = all_files
    .into_iter()
    .filter(|f| !applied.contains(f))
    .collect();

  Ok(MigrationStatus { applied, pending })
}

fn collect_all_sql_files(migrations_dir: &str) -> Result<Vec<String>, MigrateError> {
  let path = Path::new(migrations_dir);
  if !path.exists() {
    return Ok(vec![]);
  }

  let mut all_files: Vec<String> = Vec::new();
  let entries =
    std::fs::read_dir(path).map_err(|e| MigrateError::ReadDir(format!("{migrations_dir}: {e}")))?;

  for entry in entries {
    let entry = entry.map_err(|e| MigrateError::ReadDir(format!("{migrations_dir}: {e}")))?;
    let name = entry.file_name().to_string_lossy().into_owned();
    if name.ends_with(".sql") {
      all_files.push(name);
    }
  }

  all_files.sort();
  Ok(all_files)
}
