//! The borrowed-connection surface: `SqliteMaintenance` and its rusqlite impl.

use std::path::Path;

use super::attached::AttachedDatabase;
use super::error::MaintenanceError;
use super::integrity::IntegrityReport;
use super::pragma_reads;
use super::schema_name::quote_schema;
use super::storage::StorageStats;

/// Typed SQLite maintenance and inspection on a connection you own.
///
/// Implemented for `rusqlite::Connection` itself, so it works on a borrowed
/// connection: nothing here opens a database, starts a transaction, or takes
/// ownership. Bring it into scope and call it on the connection you already
/// have.
///
/// ```ignore
/// use toolu_orm_connection::SqliteMaintenance;
///
/// // A validated pre-upgrade snapshot, end to end.
/// conn.vacuum_into(Path::new("/var/app/snapshot.db"))?;
/// let snapshot = conn.attach_database(Path::new("/var/app/snapshot.db"), "snapshot")?;
/// let report = snapshot.quick_check()?;
/// snapshot.detach()?;
/// if !report.is_ok() {
///   return Err(format!("snapshot is unusable: {report}").into());
/// }
/// ```
pub trait SqliteMaintenance {
  /// `VACUUM INTO` — write a consistent copy of the whole database to
  /// `destination`.
  ///
  /// The path travels as a bound parameter, never as SQL text, so no quoting
  /// question arises and no path can alter the statement. SQLite refuses to
  /// overwrite: `destination` must not exist yet.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::NonUtf8Path`] when `destination` is not UTF-8,
  /// before any statement is sent. Returns [`MaintenanceError::Sqlite`] with
  /// SQLite's own error otherwise — `output file already exists` for a taken
  /// destination, `cannot VACUUM from within a transaction` when the caller has
  /// one open, `unable to open database file` when the path is unwritable.
  fn vacuum_into(&self, destination: &Path) -> Result<(), MaintenanceError>;

  /// `PRAGMA main.quick_check` — the integrity of the main database, and only
  /// of the main database, whatever else is attached to this connection.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma. A
  /// database that fails the check is not an error: it comes back as a report
  /// whose [`is_ok`](IntegrityReport::is_ok) is false.
  fn quick_check(&self) -> Result<IntegrityReport, MaintenanceError>;

  /// `PRAGMA main.page_count`.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma.
  fn page_count(&self) -> Result<i64, MaintenanceError>;

  /// `PRAGMA main.page_size`.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma.
  fn page_size(&self) -> Result<i64, MaintenanceError>;

  /// Both pragmas above, read in one call — the storage statistics of the main
  /// database.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if either pragma fails.
  fn storage_stats(&self) -> Result<StorageStats, MaintenanceError>;

  /// `ATTACH DATABASE` the file at `path` under `schema`.
  ///
  /// The path is bound; the schema is rendered as a properly quoted identifier,
  /// so a name containing `"` attaches the database it names rather than
  /// breaking the statement. The returned guard detaches on drop, which is what
  /// makes a failed copy safe to abandon with `?`.
  ///
  /// A path that does not exist is created as an empty database, which is
  /// SQLite's documented behavior for a writable connection.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::NonUtf8Path`] or
  /// [`MaintenanceError::InvalidSchemaName`] before any statement is sent.
  /// Returns [`MaintenanceError::Sqlite`] with SQLite's own error otherwise —
  /// `database main is already in use` for a reserved or taken name, `file is
  /// not a database` when the file is not one.
  fn attach_database<'a>(
    &'a self,
    path: &Path,
    schema: &str,
  ) -> Result<AttachedDatabase<'a>, MaintenanceError>;
}

impl SqliteMaintenance for rusqlite::Connection {
  fn vacuum_into(&self, destination: &Path) -> Result<(), MaintenanceError> {
    let destination = path_argument(destination)?;
    self.execute("VACUUM INTO ?1", [destination])?;
    Ok(())
  }

  fn quick_check(&self) -> Result<IntegrityReport, MaintenanceError> {
    pragma_reads::quick_check(self, Some(MAIN))
  }

  fn page_count(&self) -> Result<i64, MaintenanceError> {
    pragma_reads::page_count(self, Some(MAIN))
  }

  fn page_size(&self) -> Result<i64, MaintenanceError> {
    pragma_reads::page_size(self, Some(MAIN))
  }

  fn storage_stats(&self) -> Result<StorageStats, MaintenanceError> {
    pragma_reads::storage_stats(self, Some(MAIN))
  }

  fn attach_database<'a>(
    &'a self,
    path: &Path,
    schema: &str,
  ) -> Result<AttachedDatabase<'a>, MaintenanceError> {
    let path = path_argument(path)?;
    // Both refusals happen before the statement is built, so a rejected call
    // leaves the connection exactly as it found it.
    let quoted = quote_schema(schema)?;
    self.execute(&format!("ATTACH DATABASE ?1 AS {quoted}"), [path])?;
    Ok(AttachedDatabase::new(self, schema.to_owned(), quoted))
  }
}

/// Every read on this trait names `main` explicitly rather than letting SQLite
/// pick a default. It matters for `quick_check`: a schema-less
/// `PRAGMA quick_check` checks *every* attached database, so the moment an
/// [`AttachedDatabase`] is alive an unqualified call would report that
/// attachment's problems as if they were the caller's own. Naming the schema
/// makes `conn.quick_check()` mean the main database and nothing else, whatever
/// happens to be attached, and keeps it the exact counterpart of
/// [`AttachedDatabase::quick_check`].
const MAIN: &str = "main";

/// SQLite takes filenames as UTF-8 text, so a path that is not UTF-8 cannot be
/// bound at all and is refused here rather than lossily converted.
fn path_argument(path: &Path) -> Result<&str, MaintenanceError> {
  path
    .to_str()
    .ok_or_else(|| MaintenanceError::NonUtf8Path(path.to_path_buf()))
}
