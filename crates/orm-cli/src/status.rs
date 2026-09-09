//! Migration status reporting (applied vs pending).

use std::path::Path;

use toolu_orm_connection::{DbConnection, DbConnectionBlocking};
use toolu_orm_core::dialect::Dialect;

use crate::migrate::embedded::reject_duplicate_names;
use crate::migrate::{
  ensure_migrations_table, ensure_migrations_table_blocking, get_applied_migrations,
  get_applied_migrations_blocking, EmbeddedMigration, MigrateError,
};

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

/// Blocking twin of [`get_status`].
///
/// # Errors
///
/// Returns `MigrateError` on database or file I/O failures.
pub fn get_status_blocking(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  dialect: Dialect,
) -> Result<MigrationStatus, MigrateError> {
  ensure_migrations_table_blocking(conn, dialect)?;
  let applied = get_applied_migrations_blocking(conn)?;
  let all_files = collect_all_sql_files(migrations_dir)?;

  let pending: Vec<String> = all_files
    .into_iter()
    .filter(|f| !applied.contains(f))
    .collect();

  Ok(MigrationStatus { applied, pending })
}

/// Returns applied vs pending status against an embedded migration list.
///
/// `applied` comes from `_migrations`. `pending` is every name in the list that
/// is not yet recorded, in **list order** — the same order
/// [`crate::migrate::run_migrate_embedded`] would apply them.
///
/// # Errors
///
/// Returns [`MigrateError::DuplicateMigration`] when the list repeats a name,
/// or [`MigrateError::Database`] on a database failure.
pub async fn get_status_embedded(
  conn: &impl DbConnection,
  migrations: &[EmbeddedMigration<'_>],
  dialect: Dialect,
) -> Result<MigrationStatus, MigrateError> {
  reject_duplicate_names(migrations)?;

  ensure_migrations_table(conn, dialect).await?;
  let applied = get_applied_migrations(conn).await?;

  let pending: Vec<String> = migrations
    .iter()
    .map(|entry| entry.name.to_owned())
    .filter(|name| !applied.contains(name))
    .collect();

  Ok(MigrationStatus { applied, pending })
}

/// Blocking twin of [`get_status_embedded`].
///
/// # Errors
///
/// Same as [`get_status_embedded`].
pub fn get_status_embedded_blocking(
  conn: &impl DbConnectionBlocking,
  migrations: &[EmbeddedMigration<'_>],
  dialect: Dialect,
) -> Result<MigrationStatus, MigrateError> {
  reject_duplicate_names(migrations)?;

  ensure_migrations_table_blocking(conn, dialect)?;
  let applied = get_applied_migrations_blocking(conn)?;

  let pending: Vec<String> = migrations
    .iter()
    .map(|entry| entry.name.to_owned())
    .filter(|name| !applied.contains(name))
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
