//! RenameResolver trait for detecting table and column renames during schema diff.

/// Trait for resolving table and column renames during schema diff.
pub trait RenameResolver {
  /// Given lists of added and removed table names, return pairs of (old, new)
  /// names that represent renames rather than drops + creates.
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)>;

  /// Given lists of added and removed column names within a table, return pairs
  /// of (old, new) names that represent renames rather than drops + adds.
  fn resolve_columns(
    &self,
    table: &str,
    added: &[String],
    removed: &[String],
  ) -> Vec<(String, String)>;
}

/// Default resolver that never detects renames.
pub struct NoRenames;

impl RenameResolver for NoRenames {
  fn resolve_tables(&self, _added: &[String], _removed: &[String]) -> Vec<(String, String)> {
    Vec::new()
  }

  fn resolve_columns(
    &self,
    _table: &str,
    _added: &[String],
    _removed: &[String],
  ) -> Vec<(String, String)> {
    Vec::new()
  }
}
