//! Mutating a migrations directory *after* its migrations have been applied:
//! the edits an applied-history check must catch, and the prunings it must
//! tolerate. Shared by `migrate_history_test.rs` and
//! `migrate_history_blocking_test.rs`.

use toolu_orm_core::journal::Journal;

/// The migration issue #86 reproduces with.
pub const LEDGER_SQL: &str = "CREATE TABLE ledger (id INTEGER PRIMARY KEY);";
/// The same file after an edit that a plain re-run must not accept.
pub const EDITED_LEDGER_SQL: &str = "CREATE TABLE ledger (id INTEGER PRIMARY KEY, secret TEXT);";
/// A second migration, so a tampered entry can be shown to block a pending one.
pub const AUDIT_SQL: &str = "CREATE TABLE audit (id TEXT);";

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Writes (or rewrites) one migration file, leaving `_journal.json` untouched.
///
/// # Errors
///
/// Returns the file-write error.
pub fn write_file(dir: &str, name: &str, sql: &str) -> FixtureResult<()> {
  std::fs::write(format!("{dir}/{name}"), sql)?;
  Ok(())
}

/// Replaces `_journal.json` with exactly these `(name, hash)` entries, so a test
/// can declare a hash that is not the one the file hashes to.
///
/// # Errors
///
/// Returns the journal-write error.
pub fn write_journal(dir: &str, entries: &[(&str, &str)]) -> FixtureResult<()> {
  let mut journal = Journal::empty();
  for (name, hash) in entries {
    journal.add_entry(name, hash);
  }
  journal.write_to_path(&format!("{dir}/_journal.json"))?;
  Ok(())
}

/// Deletes a migration file but keeps its journal entry: a history pruned on
/// disk, as a squash or a late adopter leaves it.
///
/// # Errors
///
/// Returns the remove-file error.
pub fn prune_file(dir: &str, name: &str) -> FixtureResult<()> {
  std::fs::remove_file(format!("{dir}/{name}"))?;
  Ok(())
}

/// Replaces a migration file with a directory of the same name: a path that
/// exists and cannot be read, which must not pass for a pruned file.
///
/// # Errors
///
/// Returns the remove-file or create-dir error.
pub fn make_unreadable(dir: &str, name: &str) -> FixtureResult<()> {
  std::fs::remove_file(format!("{dir}/{name}"))?;
  std::fs::create_dir(format!("{dir}/{name}"))?;
  Ok(())
}
