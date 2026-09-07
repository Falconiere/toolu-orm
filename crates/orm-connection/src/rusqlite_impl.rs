//! rusqlite backend: sync SQLite wrapped with tokio::task::spawn_blocking.
//!
//! For consumers that run with embedded SQLite and no network database
//! (for example remote workers).

use std::sync::Arc;

use crate::error::DbError;
use crate::trait_def::DbConnection;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// A connection wrapping `rusqlite::Connection` with async interface via `spawn_blocking`.
///
/// The inner connection is wrapped in `Arc<tokio::sync::Mutex>` to satisfy
/// `Send + Sync` requirements while preventing concurrent access to the
/// single-threaded SQLite connection.
pub struct RusqliteConnection {
  inner: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
}

impl RusqliteConnection {
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
    Ok(Self {
      inner: Arc::new(tokio::sync::Mutex::new(conn)),
    })
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
    Ok(Self {
      inner: Arc::new(tokio::sync::Mutex::new(conn)),
    })
  }
}

#[async_trait::async_trait]
impl DbConnection for RusqliteConnection {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let conn = Arc::clone(&self.inner);
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || {
      let guard = conn.blocking_lock();
      let params_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();
      let affected = guard
        .execute(&sql, params_refs.as_slice())
        .map_err(|e| DbError::Query(e.to_string()))?;
      Ok::<u64, DbError>(affected as u64)
    })
    .await
    .map_err(|e| DbError::Query(e.to_string()))?
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    let conn = Arc::clone(&self.inner);
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || {
      let guard = conn.blocking_lock();
      let params_refs: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();
      let mut stmt = guard
        .prepare(&sql)
        .map_err(|e| DbError::Query(e.to_string()))?;
      let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
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
      Ok::<Vec<T>, DbError>(results)
    })
    .await
    .map_err(|e| DbError::Query(e.to_string()))?
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    let conn = Arc::clone(&self.inner);
    let sql = sql.to_owned();
    tokio::task::spawn_blocking(move || {
      let guard = conn.blocking_lock();
      guard
        .execute_batch(&sql)
        .map_err(|e| DbError::Query(e.to_string()))?;
      Ok::<(), DbError>(())
    })
    .await
    .map_err(|e| DbError::Query(e.to_string()))?
  }
}
