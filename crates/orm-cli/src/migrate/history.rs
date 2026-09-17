//! Validating the history a database has already applied.
//!
//! An applied migration is skipped on every later run, so each runner checks
//! its whole declared history here *before* applying anything: the hash
//! recorded when the migration ran against the hash its source declares now
//! ([`MigrateError::HistoryMismatch`]), then that hash against the migration's
//! current bytes ([`MigrateError::HashMismatch`]). A row recorded without a
//! hash and a pruned `.sql` file cannot be verified and say so (see
//! [`compare_recorded`] and [`read_applied_sql`]).

use std::path::Path;

use toolu_orm_core::journal::JournalEntry;

use super::apply::verify_hash;
use super::embedded::EmbeddedMigration;
use super::error::MigrateError;
use super::store::AppliedMigration;

/// Checks every journal entry the database has already applied.
///
/// Entries that are not applied yet are left to the apply path, which verifies
/// their hash before running them; rows in `_migrations` that the journal does
/// not list (a squashed or pruned history) declare nothing to compare and are
/// not validated.
///
/// Each applied entry costs one file read. That is the point: the hash the
/// journal declares can agree with the database while the file it names has
/// been edited, which is exactly the case that used to pass unnoticed.
pub(super) fn validate_directory_history(
  migrations_dir: &str,
  entries: &[JournalEntry],
  applied: &[AppliedMigration],
) -> Result<(), MigrateError> {
  for entry in entries {
    let Some(record) = recorded(applied, &entry.name) else {
      continue;
    };
    if compare_recorded(&entry.name, &record.hash, &entry.hash)? == Verdict::Unverifiable {
      continue;
    }
    let Some(sql) = read_applied_sql(migrations_dir, &entry.name)? else {
      continue;
    };
    verify_hash(&entry.name, &sql, &entry.hash)?;
  }
  Ok(())
}

/// Checks every embedded entry the database has already applied.
///
/// The embedded list is its own journal, and its SQL travels with it, so both
/// comparisons always apply: there is no pruned-file case.
pub(super) fn validate_embedded_history(
  migrations: &[EmbeddedMigration<'_>],
  applied: &[AppliedMigration],
) -> Result<(), MigrateError> {
  for migration in migrations {
    let Some(record) = recorded(applied, migration.name) else {
      continue;
    };
    if compare_recorded(migration.name, &record.hash, migration.hash)? == Verdict::Unverifiable {
      continue;
    }
    verify_hash(migration.name, migration.sql, migration.hash)?;
  }
  Ok(())
}

/// What the recorded hash allows us to say about an applied migration.
#[derive(PartialEq, Eq)]
enum Verdict {
  /// The database recorded no hash for it, so nothing about it can be proven.
  Unverifiable,
  /// The recorded hash matches what the source declares; the bytes can be
  /// checked against that hash.
  Consistent,
}

/// Compares the hash recorded when the migration ran with the one its source
/// declares today.
fn compare_recorded(name: &str, recorded: &str, declared: &str) -> Result<Verdict, MigrateError> {
  if recorded.is_empty() {
    return Ok(Verdict::Unverifiable);
  }
  if recorded == declared {
    return Ok(Verdict::Consistent);
  }
  Err(MigrateError::HistoryMismatch {
    file: name.to_owned(),
    recorded: recorded.to_owned(),
    declared: declared.to_owned(),
  })
}

fn recorded<'a>(applied: &'a [AppliedMigration], name: &str) -> Option<&'a AppliedMigration> {
  applied.iter().find(|record| record.name == name)
}

/// The current bytes of an applied migration, or `None` when its file is gone.
///
/// Pruning applied `.sql` files is legitimate — a squashed history, or a project
/// that adopted toolu-orm with [`mark_applied`](super::mark_applied) and never
/// had the old files — so an absent file leaves the recorded-against-declared
/// comparison as the whole verdict. Every other I/O failure is still an error: a
/// file that exists and cannot be read must not pass for a file that was pruned.
fn read_applied_sql(migrations_dir: &str, name: &str) -> Result<Option<String>, MigrateError> {
  let path = Path::new(migrations_dir).join(name);
  match std::fs::read_to_string(&path) {
    Ok(sql) => Ok(Some(sql)),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(MigrateError::ReadFile(format!("{}: {e}", path.display()))),
  }
}
