//! Migrations compiled into the binary instead of read from a directory.

use toolu_orm_connection::DbConnection;
use toolu_orm_core::dialect::Dialect;

use super::apply::{apply_migration, verify_hash};
use super::error::MigrateError;
use super::store::{ensure_migrations_table, get_applied_migrations};

/// One migration whose SQL is resolved at compile time, usually by
/// `include_str!`.
///
/// A single-binary distribution — `cargo install`, a tap, a `curl | sh`
/// installer — has no migrations directory on the target machine, so the bytes
/// have to travel inside the executable:
///
/// ```ignore
/// const MIGRATIONS: &[EmbeddedMigration] = &[EmbeddedMigration {
///   name: "0001_init.sql",
///   sql: include_str!("../migrations/0001_init.sql"),
///   hash: "sha256:2c8f…",
/// }];
/// ```
///
/// Baking the SQL in does not weaken the integrity check, it strengthens it:
/// on disk the file can be edited after install, whereas here
/// [`verify_hash`](Self::verify_hash) turns an edit to a shipped `.sql` into a
/// failing test in the project that ships it.
pub struct EmbeddedMigration<'a> {
  /// Recorded in `_migrations.name`, and how an already-applied migration is
  /// recognized. Conventionally the file name, e.g. `"0001_init.sql"`, so a
  /// project can move between this and [`run_migrate`](super::run_migrate).
  pub name: &'a str,
  /// The migration's SQL. Several statements separate with
  /// `--> statement-breakpoint`, exactly as in a generated file.
  pub sql: &'a str,
  /// The expected SHA-256 of `sql`, as `"sha256:…"` — the same string
  /// `_journal.json` carries for this migration.
  pub hash: &'a str,
}

impl EmbeddedMigration<'_> {
  /// Checks `sql` against `hash` without touching a database, so a project can
  /// assert its whole list in one `#[test]`.
  ///
  /// # Errors
  ///
  /// Returns [`MigrateError::HashMismatch`] when the two disagree.
  pub fn verify_hash(&self) -> Result<(), MigrateError> {
    verify_hash(self.name, self.sql, self.hash)
  }
}

/// Applies pending migrations from an in-memory list instead of a directory.
///
/// The counterpart to [`run_migrate`](super::run_migrate), and identical to it
/// below the byte-fetch: same `_migrations` bookkeeping, same hash check, same
/// one-transaction-per-migration boundary. A database migrated from a directory
/// and one migrated from the equivalent list are indistinguishable, so a
/// project can switch sources between releases.
///
/// Migrations apply in slice order, which is the caller's declaration of order
/// just as `_journal.json` is on disk — they are not sorted by name. Entries
/// already in `_migrations` are skipped, so the count is how many were newly
/// applied.
///
/// # Errors
///
/// Returns [`MigrateError::DuplicateMigration`] when two entries share a name,
/// before anything is written; [`MigrateError::HashMismatch`] when an entry's
/// SQL no longer matches its declared hash, leaving earlier entries applied;
/// or [`MigrateError::Database`] when a statement fails, rolling that migration
/// back whole.
pub async fn run_migrate_embedded(
  conn: &impl DbConnection,
  migrations: &[EmbeddedMigration<'_>],
  dialect: Dialect,
) -> Result<u32, MigrateError> {
  reject_duplicate_names(migrations)?;

  ensure_migrations_table(conn, dialect).await?;
  let applied = get_applied_migrations(conn).await?;

  let mut count: u32 = 0;
  for migration in migrations {
    if applied.iter().any(|name| name == migration.name) {
      continue;
    }
    apply_migration(conn, migration.name, migration.sql, migration.hash, dialect).await?;
    count += 1;
  }

  Ok(count)
}

/// Records `names` as already applied from an embedded list, without executing
/// their SQL.
///
/// The list is the journal: hashes come from each [`EmbeddedMigration::hash`],
/// selection follows list order, and names absent from the list fail with
/// [`MigrateError::NotInJournal`] before anything is written — the same
/// contract as [`super::mark_applied`] against `_journal.json`.
///
/// # Errors
///
/// Returns [`MigrateError::DuplicateMigration`] when the list repeats a name,
/// [`MigrateError::NotInJournal`] when any requested name is missing, or
/// [`MigrateError::Database`] on a database failure.
pub async fn mark_applied_embedded(
  conn: &impl DbConnection,
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

  super::baseline::record_all(conn, &selected, dialect).await
}

/// Records every embedded entry up to and including `last_name` as applied,
/// without executing their SQL.
///
/// List order is the journal order. Skipping, counting, and atomicity match
/// [`mark_applied_embedded`].
///
/// # Errors
///
/// Same as [`mark_applied_embedded`]; [`MigrateError::NotInJournal`] when
/// `last_name` itself is absent from the list.
pub async fn mark_applied_through_embedded(
  conn: &impl DbConnection,
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

  super::baseline::record_all(conn, &selected, dialect).await
}

/// A hand-written list can repeat a name where a generated journal cannot, and
/// the repeat has no single SQL body. Caught before the first statement runs,
/// so the mistake does not surface as a `UNIQUE` violation with earlier
/// migrations already committed.
pub(crate) fn reject_duplicate_names(
  migrations: &[EmbeddedMigration<'_>],
) -> Result<(), MigrateError> {
  let mut seen: Vec<&str> = Vec::with_capacity(migrations.len());
  let mut duplicates: Vec<&str> = Vec::new();

  for migration in migrations {
    if seen.contains(&migration.name) {
      if !duplicates.contains(&migration.name) {
        duplicates.push(migration.name);
      }
    } else {
      seen.push(migration.name);
    }
  }

  if duplicates.is_empty() {
    return Ok(());
  }
  Err(MigrateError::DuplicateMigration(duplicates.join(", ")))
}
