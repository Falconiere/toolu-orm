//! The documented boundary of the supported surface: borrowing the driver
//! connection back out of the wrapper.

use super::connection::RusqliteConnection;
use crate::error::DbError;

impl RusqliteConnection {
  /// Borrow the wrapped `rusqlite::Connection` for the duration of one closure.
  ///
  /// This is the sanctioned escape hatch for work that is deliberately outside
  /// this ORM's supported surface, and the only way to reach the driver again
  /// after [`from_connection`](Self::from_connection) has taken ownership. Two
  /// uses are expected:
  ///
  /// - **The SQLite maintenance surface.** [`SqliteMaintenance`](crate::SqliteMaintenance)
  ///   is implemented for `rusqlite::Connection`, so
  ///   `conn.with_raw_connection(|c| c.storage_stats())?` reads the wrapper's
  ///   own database. The outer `Result` is the lock, the inner one the
  ///   operation.
  /// - **FFI that no typed API can express.** Registering a custom FTS5
  ///   tokenizer needs `SELECT fts5(?1)` bound with
  ///   `sqlite3_bind_pointer(.., "fts5_api_ptr", ..)` and then a C function
  ///   pointer out of `fts5_api`. A host pointer is not a value of any SQL
  ///   type, so neither [`Value`](toolu_orm_core::value::Value) nor rusqlite's
  ///   `ToSql` can carry it, and supporting it here would mean `unsafe` in a
  ///   workspace that denies it. That handshake is therefore **outside** the
  ///   supported surface by decision, not by omission — do it on your own
  ///   connection, either before `from_connection` or through this method.
  ///   Everything downstream of registration stays fully supported: the
  ///   `#[fts5_table]` schema, `MATCH`, `bm25`, `snippet` and `highlight`.
  ///
  /// The connection lock is held for the whole closure and it is not
  /// re-entrant, so the closure must not call back into this wrapper — doing so
  /// deadlocks. Keep the body short and hand the result back out.
  ///
  /// # Errors
  ///
  /// Returns [`DbError::Connection`] when the connection lock is poisoned by an
  /// earlier panic, exactly as every other method on this wrapper does. The
  /// closure is not run in that case.
  pub fn with_raw_connection<T>(
    &self,
    f: impl FnOnce(&rusqlite::Connection) -> T,
  ) -> Result<T, DbError> {
    let guard = self.lock()?;
    Ok(f(&guard))
  }
}
