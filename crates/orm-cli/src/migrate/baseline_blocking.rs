//! Blocking baseline APIs over [`DbConnectionBlocking`].

use std::path::Path;

use toolu_orm_connection::DbConnectionBlocking;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::Journal;

use super::error::MigrateError;
use super::store::{
  ensure_migrations_table_blocking, get_applied_migrations_blocking, record_migration_blocking,
};
use super::transaction_blocking::{begin, commit, rollback_after};

/// Blocking twin of [`super::mark_applied`].
///
/// # Errors
///
/// Same as [`super::mark_applied`].
pub fn mark_applied_blocking(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  names: &[&str],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  let journal = read_journal(migrations_dir)?;

  let unknown: Vec<&str> = names
    .iter()
    .copied()
    .filter(|name| !journal.entries.iter().any(|entry| entry.name == *name))
    .collect();
  if !unknown.is_empty() {
    return Err(MigrateError::NotInJournal(unknown.join(", ")));
  }

  let selected: Vec<(&str, &str)> = journal
    .entries
    .iter()
    .filter(|entry| names.contains(&entry.name.as_str()))
    .map(|entry| (entry.name.as_str(), entry.hash.as_str()))
    .collect();

  record_all(conn, &selected, dialect)
}

/// Blocking twin of [`super::mark_applied_through`].
///
/// # Errors
///
/// Same as [`super::mark_applied_through`].
pub fn mark_applied_through_blocking(
  conn: &impl DbConnectionBlocking,
  migrations_dir: &str,
  last_name: &str,
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  let journal = read_journal(migrations_dir)?;

  let position = journal
    .entries
    .iter()
    .position(|entry| entry.name == last_name)
    .ok_or_else(|| MigrateError::NotInJournal(last_name.to_owned()))?;

  let selected: Vec<(&str, &str)> = journal
    .entries
    .iter()
    .take(position + 1)
    .map(|entry| (entry.name.as_str(), entry.hash.as_str()))
    .collect();

  record_all(conn, &selected, dialect)
}

fn read_journal(migrations_dir: &str) -> Result<Journal, MigrateError> {
  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().ok_or_else(|| {
    MigrateError::ReadFile(format!("{} is not valid UTF-8", journal_path.display()))
  })?;
  Journal::read_from_path(journal_path_str).map_err(|e| MigrateError::ReadFile(format!("{e}")))
}

/// Records `(name, hash)` pairs in one transaction, skipping those already in
/// `_migrations`. Shared by directory and embedded blocking baseline twins.
pub(super) fn record_all(
  conn: &impl DbConnectionBlocking,
  entries: &[(&str, &str)],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  ensure_migrations_table_blocking(conn, dialect)?;

  begin(conn)?;

  let mut count: u32 = 0;
  let result = (|| {
    let applied = get_applied_migrations_blocking(conn)?;
    for &(name, hash) in entries {
      if applied.iter().any(|recorded| recorded == name) {
        continue;
      }
      record_migration_blocking(conn, name, hash, dialect)?;
      count += 1;
    }
    Ok(())
  })();

  if let Err(e) = result {
    return Err(rollback_after(conn, e));
  }

  commit(conn)?;
  Ok(count)
}
