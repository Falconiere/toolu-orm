//! A real directory on disk for the suites that need real SQLite files.
//!
//! `VACUUM INTO` and `ATTACH DATABASE` name files, so an in-memory database
//! cannot stand in for either. Wired into `rusqlite_maintenance_test` with
//! `#[path]`; every item here is used by that binary (no dead code under
//! `-D warnings`).
//!
//! No `tempfile` dependency: `std::fs` already does everything this needs, and
//! a dev-dependency that only makes a directory would be one more crate in the
//! lockfile for nothing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// What every test in this suite returns, so failures propagate with `?`
/// instead of being unwrapped.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A directory under the system temp directory, removed when this value drops.
pub struct TempDbDir {
  path: PathBuf,
}

impl TempDbDir {
  /// Create a directory whose name ends with `label`.
  ///
  /// `label` goes into the directory name verbatim, which is how this suite
  /// exercises a path holding quote characters: nothing sanitizes it, so the
  /// awkward path is the one that reaches SQLite.
  ///
  /// # Errors
  ///
  /// Returns the I/O error if the directory cannot be created.
  pub fn new(label: &str) -> Result<Self, std::io::Error> {
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
      "toolu-orm-maint-{}-{unique}-{label}",
      std::process::id()
    ));
    std::fs::create_dir_all(&path)?;
    Ok(Self { path })
  }

  /// The directory itself.
  pub fn path(&self) -> &Path {
    &self.path
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

/// Open a real on-disk database holding a `t(a TEXT NOT NULL)` table with
/// `rows` rows, valued `row-0`, `row-1`, ...
///
/// # Errors
///
/// Returns the rusqlite error if the database cannot be opened or seeded.
pub fn seeded_db(path: &Path, rows: usize) -> Result<rusqlite::Connection, rusqlite::Error> {
  let conn = rusqlite::Connection::open(path)?;
  conn.execute_batch("CREATE TABLE t (a TEXT NOT NULL)")?;
  for row in 0..rows {
    conn.execute("INSERT INTO t (a) VALUES (?1)", [format!("row-{row}")])?;
  }
  Ok(conn)
}

/// Every value in `t.a`, in insertion order.
///
/// # Errors
///
/// Returns the rusqlite error if the read fails.
pub fn rows_of(conn: &rusqlite::Connection) -> Result<Vec<String>, rusqlite::Error> {
  let mut stmt = conn.prepare("SELECT a FROM t ORDER BY rowid")?;
  let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
  rows.collect()
}

/// The schema names currently attached to `conn`, `main` first.
///
/// # Errors
///
/// Returns the rusqlite error if the read fails.
pub fn attached_schemas(conn: &rusqlite::Connection) -> Result<Vec<String>, rusqlite::Error> {
  let mut stmt = conn.prepare("SELECT name FROM pragma_database_list ORDER BY seq")?;
  let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
  rows.collect()
}
