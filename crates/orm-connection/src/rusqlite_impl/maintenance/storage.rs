//! `PRAGMA page_count` and `PRAGMA page_size` read together.

/// A database's size, as SQLite counts it.
///
/// SQLite reports both numbers as signed integers, and they are what the file
/// on disk is made of: under the default rollback journal, `page_count *
/// page_size` is exactly the main database file's length once the last write
/// has committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageStats {
  /// Pages the database currently holds, including free ones.
  pub page_count: i64,
  /// Bytes per page, fixed when the database was created.
  pub page_size: i64,
}

impl StorageStats {
  /// `page_count * page_size`, saturating instead of overflowing.
  ///
  /// Saturation is unreachable in practice — it would take an exabyte-scale
  /// database — but it keeps this a total function, so no caller has to handle
  /// an error that no real file can produce.
  #[must_use]
  pub fn bytes(self) -> i64 {
    self.page_count.saturating_mul(self.page_size)
  }
}
