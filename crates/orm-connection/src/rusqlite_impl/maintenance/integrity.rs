//! What `PRAGMA quick_check` reported about a database.

use std::fmt;

/// The rows `PRAGMA quick_check` returned, in order and unchanged.
///
/// A healthy database answers with exactly one row, `ok`. Anything else is a
/// list of problems in SQLite's own words, which is why the rows are kept
/// verbatim instead of being reduced to a boolean: `NULL value in q.b` tells a
/// caller which column to look at, and no derived type could say it better.
///
/// An unhealthy database is a *report*, not an error. Deciding that a snapshot
/// is unusable is the caller's policy, not this crate's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityReport {
  messages: Vec<String>,
}

impl IntegrityReport {
  pub(super) fn new(messages: Vec<String>) -> Self {
    Self { messages }
  }

  /// True only when SQLite returned exactly one row and it is `ok`.
  ///
  /// A pragma that returned no rows at all is therefore *not* ok: an empty
  /// answer is an unanswered question, not a clean bill of health.
  #[must_use]
  pub fn is_ok(&self) -> bool {
    matches!(self.messages.as_slice(), [only] if only == "ok")
  }

  /// Every row SQLite returned, in order, unchanged.
  #[must_use]
  pub fn messages(&self) -> &[String] {
    &self.messages
  }
}

impl fmt::Display for IntegrityReport {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.messages.is_empty() {
      // Defensive: SQLite always answers this pragma with at least one row, but
      // an empty report must still render as something a log can be read from.
      return f.write_str("quick_check returned no rows");
    }
    f.write_str(&self.messages.join("; "))
  }
}
