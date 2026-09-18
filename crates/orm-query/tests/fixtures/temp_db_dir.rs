//! A real directory on disk for the suites that need real SQLite files.
//!
//! `ATTACH DATABASE` names a file, so an in-memory database cannot stand in for
//! the source of a cross-database copy. Wired into
//! `rusqlite_insert_select_test` with `#[path]`.
//!
//! No `tempfile` dependency: `std::fs` already does everything this needs, and
//! a dev-dependency that only makes a directory would be one more crate in the
//! lockfile for nothing. The same pattern is in `toolu-orm-connection`'s own
//! `tests/fixtures/temp_db_dir.rs`; the two crates cannot share a fixture file.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A directory under the system temp directory, removed when this value drops.
pub struct TempDbDir {
  path: PathBuf,
}

impl TempDbDir {
  /// Create a directory whose name ends with `label`.
  ///
  /// # Errors
  ///
  /// Returns the I/O error if the directory cannot be created.
  pub fn new(label: &str) -> Result<Self, std::io::Error> {
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
      "toolu-orm-copy-{}-{unique}-{label}",
      std::process::id()
    ));
    std::fs::create_dir_all(&path)?;
    Ok(Self { path })
  }

  /// A path inside the directory. The file is not created.
  pub fn file(&self, name: &str) -> PathBuf {
    self.path.join(name)
  }
}

impl Drop for TempDbDir {
  fn drop(&mut self) {
    // A test that already failed must not be masked by a cleanup failure.
    let _ = std::fs::remove_dir_all(&self.path);
  }
}
