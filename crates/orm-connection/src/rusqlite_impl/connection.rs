//! The connection wrapper: shared state, constructors, and the two ways in --
//! the connection lock for statements and the admission gate for async callers.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::DbError;

/// A connection wrapping `rusqlite::Connection`, usable synchronously through
/// [`crate::DbConnectionBlocking`] and asynchronously through
/// [`crate::DbConnection`].
///
/// The inner connection is wrapped in `Arc<std::sync::Mutex>` to satisfy
/// `Send + Sync` requirements while preventing concurrent access to the
/// single-threaded SQLite connection. The lock is a std one, not a tokio one,
/// because the blocking methods must be callable from anywhere -- including
/// from inside a runtime, where `tokio::sync::Mutex::blocking_lock` panics.
/// The async methods additionally queue on a shared one-permit gate before
/// entering tokio's blocking pool, so they wait on their own tasks instead.
pub struct RusqliteConnection {
  inner: Arc<Mutex<rusqlite::Connection>>,
  admission: Arc<tokio::sync::Semaphore>,
}

impl RusqliteConnection {
  /// Adopt an already-open `rusqlite::Connection`.
  ///
  /// The connection is taken as-is, so anything established on it beforehand --
  /// pragmas such as `foreign_keys` or `journal_mode`, open flags, a loaded
  /// extension, an attached database -- stays in effect. Use this when the
  /// connection needs configuration that `open` cannot express.
  ///
  /// Wrapping cannot fail or block, so this is neither `async` nor fallible;
  /// a connection that is unusable surfaces at the first statement as
  /// `DbError::Query`. It is also the runtime-free way in: paired with
  /// [`crate::DbConnectionBlocking`], no tokio runtime is involved at any point.
  #[must_use]
  pub fn from_connection(conn: rusqlite::Connection) -> Self {
    Self {
      inner: Arc::new(Mutex::new(conn)),
      // One permit, because one connection runs one statement at a time: a
      // second admitted caller could only sit on the mutex holding a
      // blocking-pool thread, which is the starvation this gate exists to stop.
      admission: Arc::new(tokio::sync::Semaphore::new(1)),
    }
  }

  /// Open an in-memory SQLite database. Useful for tests.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the database cannot be opened.
  pub async fn open_in_memory() -> Result<Self, DbError> {
    let conn = tokio::task::spawn_blocking(|| {
      rusqlite::Connection::open_in_memory().map_err(|e| DbError::Connection(e.to_string()))
    })
    .await
    .map_err(|e| DbError::Connection(e.to_string()))??;
    Ok(Self::from_connection(conn))
  }

  /// Open a SQLite database file.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the file cannot be opened.
  pub async fn open(path: &str) -> Result<Self, DbError> {
    let path = path.to_owned();
    let conn = tokio::task::spawn_blocking(move || {
      rusqlite::Connection::open(&path).map_err(|e| DbError::Connection(e.to_string()))
    })
    .await
    .map_err(|e| DbError::Connection(e.to_string()))??;
    Ok(Self::from_connection(conn))
  }

  /// A second wrapper over the same connection, owned so it can be moved into a
  /// `spawn_blocking` closure. Private: sharing one connection between two live
  /// wrappers is an implementation detail of the async delegation, not a promise.
  ///
  /// Both the connection and the admission gate are shared, so every handle to
  /// one connection queues on the same permit.
  pub(super) fn handle(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
      admission: Arc::clone(&self.admission),
    }
  }

  /// Take the connection lock.
  ///
  /// A poisoned lock means an earlier caller panicked mid-statement, so the
  /// connection may be left mid-transaction; it is reported rather than
  /// recovered.
  pub(super) fn lock(&self) -> Result<MutexGuard<'_, rusqlite::Connection>, DbError> {
    self.inner.lock().map_err(|e| {
      DbError::Connection(format!(
        "the rusqlite connection lock is poisoned by an earlier panic: {e}"
      ))
    })
  }

  /// Wait for this connection's turn before any blocking thread is spent on it.
  ///
  /// The permit is owned, so the caller can move it into the blocking closure
  /// and keep it held for as long as the statement actually runs.
  pub(super) async fn admit(&self) -> Result<tokio::sync::OwnedSemaphorePermit, DbError> {
    Arc::clone(&self.admission)
      .acquire_owned()
      .await
      .map_err(|e| {
        DbError::Connection(format!(
          "the rusqlite connection admission gate is closed: {e}"
        ))
      })
  }
}
