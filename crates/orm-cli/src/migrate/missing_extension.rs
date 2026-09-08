//! Map a driver's `no such module: <name>` into [`MigrateError::MissingExtension`].
//!
//! `vec0` (and any other loadable module) is not part of SQLite; without the
//! extension registered on the connection the prepare fails with that phrasing
//! and nothing else to tell the operator what to do. The match is substring-
//! based because drivers wrap the SQLite text differently — rusqlite reports
//! it bare, libsql as `SQLite failure: \`no such module: vec0\``.

use toolu_orm_connection::DbError;

use super::error::MigrateError;

const MARKER: &str = "no such module: ";

/// Prefer [`MigrateError::MissingExtension`] when `err` names a missing module;
/// otherwise keep the existing `Database` wrapping with the migration file.
pub(super) fn map_statement_error(file: &str, err: &DbError) -> MigrateError {
  MigrateError::from_statement(file, &err.to_string())
}

impl MigrateError {
  /// Classify a statement failure from the driver's display text.
  ///
  /// Used by the migration runner and by the suite that pins the empty-name
  /// boundary a real database cannot produce.
  #[must_use]
  pub fn from_statement(file: &str, message: &str) -> Self {
    match module_name(message) {
      Some(module) => Self::MissingExtension {
        file: file.to_owned(),
        module: module.to_owned(),
      },
      None => Self::Database(format!("{file}: {message}")),
    }
  }
}

/// The identifier after the first `no such module: `, if any. Trailing
/// punctuation (a closing backtick from libsql, a period, …) is stripped so
/// the reported module name is the bare identifier the operator loads.
fn module_name(message: &str) -> Option<&str> {
  let after = message.split_once(MARKER)?.1.trim();
  let end = after
    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
    .unwrap_or(after.len());
  let name = after.get(..end)?.trim();
  if name.is_empty() {
    None
  } else {
    Some(name)
  }
}
