//! Blocking embedded migrate / baseline over [`DbConnectionBlocking`].

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;

use super::apply_blocking::apply_migration;
use super::embedded::{reject_duplicate_names, EmbeddedMigration};
use super::error::MigrateError;
use super::store::{ensure_migrations_table_blocking, get_applied_migrations_blocking};

/// Blocking twin of [`super::run_migrate_embedded`].
///
/// # Errors
///
/// Same as [`super::run_migrate_embedded`].
pub fn run_migrate_embedded_blocking(
  conn: &impl DbConnectionBlocking,
  migrations: &[EmbeddedMigration<'_>],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  reject_duplicate_names(migrations)?;

  ensure_migrations_table_blocking(conn, dialect)?;
  let applied = get_applied_migrations_blocking(conn)?;

  let mut count: u32 = 0;
  for migration in migrations {
    if applied.iter().any(|name| name == migration.name) {
      continue;
    }
    apply_migration(conn, migration.name, migration.sql, migration.hash, dialect)?;
    count += 1;
  }

  Ok(count)
}

/// Blocking twin of [`super::mark_applied_embedded`].
///
/// # Errors
///
/// Same as [`super::mark_applied_embedded`].
pub fn mark_applied_embedded_blocking(
  conn: &impl DbConnectionBlocking,
  migrations: &[EmbeddedMigration<'_>],
  names: &[&str],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  reject_duplicate_names(migrations)?;

  let unknown: Vec<&str> = names
    .iter()
    .copied()
    .filter(|name| !migrations.iter().any(|entry| entry.name == *name))
    .collect();
  if !unknown.is_empty() {
    return Err(MigrateError::NotInJournal(unknown.join(", ")));
  }

  let selected: Vec<(&str, &str)> = migrations
    .iter()
    .filter(|entry| names.contains(&entry.name))
    .map(|entry| (entry.name, entry.hash))
    .collect();

  super::baseline_blocking::record_all(conn, &selected, dialect)
}

/// Blocking twin of [`super::mark_applied_through_embedded`].
///
/// # Errors
///
/// Same as [`super::mark_applied_through_embedded`].
pub fn mark_applied_through_embedded_blocking(
  conn: &impl DbConnectionBlocking,
  migrations: &[EmbeddedMigration<'_>],
  last_name: &str,
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  reject_duplicate_names(migrations)?;

  let position = migrations
    .iter()
    .position(|entry| entry.name == last_name)
    .ok_or_else(|| MigrateError::NotInJournal(last_name.to_owned()))?;

  let selected: Vec<(&str, &str)> = migrations
    .iter()
    .take(position + 1)
    .map(|entry| (entry.name, entry.hash))
    .collect();

  super::baseline_blocking::record_all(conn, &selected, dialect)
}
