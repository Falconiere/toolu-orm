//! rusqlite backend: sync SQLite, native for `DbConnectionBlocking` and wrapped
//! with `tokio::task::spawn_blocking` for the async `DbConnection`.
//!
//! For consumers that run with embedded SQLite and no network database
//! (for example remote workers).

use std::sync::{Arc, Mutex, MutexGuard};

use crate::blocking_trait_def::DbConnectionBlocking;
use crate::error::DbError;
use crate::trait_def::DbConnection;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// A connection wrapping `rusqlite::Connection`, usable synchronously through
/// [`DbConnectionBlocking`] and asynchronously through [`DbConnection`].
///
/// The inner connection is wrapped in `Arc<std::sync::Mutex>` to satisfy
/// `Send + Sync` requirements while preventing concurrent access to the
/// single-threaded SQLite connection. The lock is a std one, not a tokio one,
/// because the blocking methods must be callable from anywhere -- including
/// from inside a runtime, where `tokio::sync::Mutex::blocking_lock` panics.
pub struct RusqliteConnection {
  inner: Arc<Mutex<rusqlite::Connection>>,
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
  /// [`DbConnectionBlocking`], no tokio runtime is involved at any point.
  #[must_use]
  pub fn from_connection(conn: rusqlite::Connection) -> Self {
    Self {
      inner: Arc::new(Mutex::new(conn)),
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
  fn handle(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
    }
  }

  /// Take the connection lock.
  ///
  /// A poisoned lock means an earlier caller panicked mid-statement, so the
  /// connection may be left mid-transaction; it is reported rather than
  /// recovered.
  fn lock(&self) -> Result<MutexGuard<'_, rusqlite::Connection>, DbError> {
    self.inner.lock().map_err(|e| {
      DbError::Connection(format!(
        "the rusqlite connection lock is poisoned by an earlier panic: {e}"
      ))
    })
  }
}

/// Borrow the parameters as rusqlite's dynamic parameter type.
fn to_sql_params(params: &[Value]) -> Vec<&dyn rusqlite::types::ToSql> {
  params
    .iter()
    .map(|v| v as &dyn rusqlite::types::ToSql)
    .collect()
}

impl DbConnectionBlocking for RusqliteConnection {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let guard = self.lock()?;
    let affected = guard
      .execute(sql, to_sql_params(&params).as_slice())
      .map_err(|e| DbError::Query(e.to_string()))?;
    Ok(affected as u64)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError> {
    let guard = self.lock()?;
    let mut stmt = guard
      .prepare(sql)
      .map_err(|e| DbError::Query(e.to_string()))?;
    let rows = stmt
      .query_map(to_sql_params(&params).as_slice(), |row| {
        #[cfg(all(feature = "rusqlite", any(feature = "postgres", feature = "libsql"),))]
        let mapped = T::from_rusqlite_row(row);
        #[cfg(all(
          feature = "rusqlite",
          not(feature = "postgres"),
          not(feature = "libsql"),
        ))]
        let mapped = T::from_row(row);
        mapped.map_err(|e| {
          rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Null, Box::new(e))
        })
      })
      .map_err(|e| DbError::Query(e.to_string()))?;
    let mut results = Vec::new();
    for row_result in rows {
      results.push(row_result.map_err(|e| DbError::Query(e.to_string()))?);
    }
    Ok(results)
  }

  fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    let guard = self.lock()?;
    guard
      .execute_batch(sql)
      .map_err(|e| DbError::Query(e.to_string()))
  }
}

/// Every method hands off to its [`DbConnectionBlocking`] twin on a blocking
/// thread, so the two surfaces run the same statement code and cannot drift.
#[async_trait::async_trait]
impl DbConnection for RusqliteConnection {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let handle = self.handle();
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || DbConnectionBlocking::execute_sql(&handle, &sql, params))
      .await
      .map_err(|e| DbError::Query(e.to_string()))?
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    let handle = self.handle();
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || DbConnectionBlocking::query_map(&handle, &sql, params))
      .await
      .map_err(|e| DbError::Query(e.to_string()))?
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    let handle = self.handle();
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || DbConnectionBlocking::execute_batch(&handle, &sql))
      .await
      .map_err(|e| DbError::Query(e.to_string()))?
  }
}
