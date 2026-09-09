//! Recording migrations as applied without executing them, so a database whose
//! schema is already at some version can adopt toolu-orm.

use std::path::Path;

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::journal::Journal;

use super::error::MigrateError;
use super::store::{ensure_migrations_table, get_applied_migrations, record_migration};
use super::transaction::{begin, commit, rollback_after};

/// Records `names` as already applied without executing their SQL.
///
/// For adopting toolu-orm on a database whose schema was established by a prior
/// migration system. Hashes come from `_journal.json`, so a later
/// [`run_migrate`](super::run_migrate) still detects a tampered file. Names
/// already recorded are skipped, so a repeated baseline is a no-op; the count
/// is the number of rows newly recorded.
///
/// The migration files themselves are never read: the journal is the integrity
/// record, and an adopting project may no longer have every historical `.sql`
/// on disk.
///
/// # Errors
///
/// Returns [`MigrateError::NotInJournal`] when any name has no journal entry —
/// nothing is recorded in that case — [`MigrateError::ReadFile`] when the
/// journal cannot be read, or [`MigrateError::Database`] on a database failure.
pub async fn mark_applied(
  conn: &impl DbConnection,
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

  // Iterating the journal rather than `names` records in journal order, so
  // `_migrations.id` order keeps matching it, and repeats collapse.
  let selected: Vec<(&str, &str)> = journal
    .entries
    .iter()
    .filter(|entry| names.contains(&entry.name.as_str()))
    .map(|entry| (entry.name.as_str(), entry.hash.as_str()))
    .collect();

  record_all(conn, &selected, dialect).await
}

/// Records every journal entry up to and including `last_name` as applied,
/// without executing their SQL.
///
/// The usual adoption shape: a project knows "my database is at 0016", not the
/// list of sixteen file names. Skipping, counting, and atomicity match
/// [`mark_applied`].
///
/// # Errors
///
/// Same as [`mark_applied`]; [`MigrateError::NotInJournal`] when `last_name`
/// itself has no journal entry.
pub async fn mark_applied_through(
  conn: &impl DbConnection,
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

  record_all(conn, &selected, dialect).await
}

fn read_journal(migrations_dir: &str) -> Result<Journal, MigrateError> {
  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().ok_or_else(|| {
    MigrateError::ReadFile(format!("{} is not valid UTF-8", journal_path.display()))
  })?;
  Journal::read_from_path(journal_path_str).map_err(|e| MigrateError::ReadFile(format!("{e}")))
}

/// Records `(name, hash)` pairs in one transaction, skipping those
/// `_migrations` already holds (its `name` column is UNIQUE, so re-inserting
/// would fail).
///
/// The already-applied set is read *inside* the transaction, so the skip
/// decision and the inserts see one state of the table. A baseline racing
/// another writer on the same names still loses on the `UNIQUE` constraint, and
/// then rolls back whole: no partial baseline, and the retry records nothing.
///
/// Shared by directory [`mark_applied`] and the embedded twins in
/// [`super::embedded`].
pub(crate) async fn record_all(
  conn: &impl DbConnection,
  entries: &[(&str, &str)],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  ensure_migrations_table(conn, dialect).await?;

  begin(conn).await?;

  let mut count: u32 = 0;
  let result = async {
    let applied = get_applied_migrations(conn).await?;
    for (name, hash) in entries {
      if applied.iter().any(|recorded| recorded == name) {
        continue;
      }
      record_migration(conn, name, hash, dialect).await?;
      count += 1;
    }
    Ok(())
  }
  .await;

  if let Err(e) = result {
    return Err(rollback_after(conn, e).await);
  }

  commit(conn).await?;
  Ok(count)
}
