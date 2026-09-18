//! A database attached to a borrowed connection, detached when the guard drops.

use super::error::MaintenanceError;
use super::integrity::IntegrityReport;
use super::pragma_reads;
use super::storage::StorageStats;

/// A live `ATTACH DATABASE`, removed again when this value goes out of scope.
///
/// This is the whole point of the type: the `DETACH` happens on *every* exit
/// path, including the `?` that abandons a half-finished copy, without the
/// caller writing an error branch for it. It borrows the connection, so it can
/// never outlive the connection it attached to.
///
/// `Drop` has nowhere to return a failure, so a detach that fails there is
/// logged at `warn` and the program continues. A caller that needs to *see*
/// that failure calls [`detach`](Self::detach) instead, which runs the same
/// statement and hands back its result; `Drop` then does nothing, so `DETACH`
/// is issued once either way.
///
/// SQLite refuses to detach a database that the caller's own open transaction
/// has written to (`database "<schema>" is locked`), so finish the transaction
/// before the guard goes out of scope.
pub struct AttachedDatabase<'a> {
  conn: &'a rusqlite::Connection,
  schema: String,
  quoted: String,
  detached: bool,
}

impl<'a> AttachedDatabase<'a> {
  pub(super) fn new(conn: &'a rusqlite::Connection, schema: String, quoted: String) -> Self {
    Self {
      conn,
      schema,
      quoted,
      detached: false,
    }
  }

  /// The schema name this database is attached under, exactly as it was given.
  #[must_use]
  pub fn schema(&self) -> &str {
    &self.schema
  }

  /// `PRAGMA <schema>.quick_check` — the integrity of *this* database.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma. An
  /// unhealthy database is not an error: it comes back as a report whose
  /// [`is_ok`](IntegrityReport::is_ok) is false.
  pub fn quick_check(&self) -> Result<IntegrityReport, MaintenanceError> {
    pragma_reads::quick_check(self.conn, Some(&self.schema))
  }

  /// `PRAGMA <schema>.page_count`.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma.
  pub fn page_count(&self) -> Result<i64, MaintenanceError> {
    pragma_reads::page_count(self.conn, Some(&self.schema))
  }

  /// `PRAGMA <schema>.page_size`.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if SQLite cannot run the pragma.
  pub fn page_size(&self) -> Result<i64, MaintenanceError> {
    pragma_reads::page_size(self.conn, Some(&self.schema))
  }

  /// Both pragmas above, read in one call.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] if either pragma fails.
  pub fn storage_stats(&self) -> Result<StorageStats, MaintenanceError> {
    pragma_reads::storage_stats(self.conn, Some(&self.schema))
  }

  /// Detach now and report the outcome.
  ///
  /// The guard is consumed either way, and `Drop` issues nothing afterwards —
  /// so a failure here is final and is not retried behind the caller's back.
  ///
  /// # Errors
  ///
  /// Returns [`MaintenanceError::Sqlite`] with SQLite's own message: `database
  /// "<schema>" is locked` when the caller's transaction still holds it, or
  /// `no such database` when something else already detached it.
  pub fn detach(mut self) -> Result<(), MaintenanceError> {
    self.detached = true;
    self.run_detach()
  }

  fn run_detach(&self) -> Result<(), MaintenanceError> {
    self
      .conn
      .execute_batch(&format!("DETACH DATABASE {}", self.quoted))?;
    Ok(())
  }
}

impl Drop for AttachedDatabase<'_> {
  fn drop(&mut self) {
    if self.detached {
      return;
    }
    if let Err(error) = self.run_detach() {
      tracing::warn!(
        schema = self.schema.as_str(),
        error = error.to_string().as_str(),
        "detaching the database failed; it is still attached to this connection"
      );
    }
  }
}
