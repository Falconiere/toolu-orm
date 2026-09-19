//! The synchronous surface: take the connection lock, run the statement,
//! return. No runtime, and no admission gate -- a synchronous caller cannot
//! await one, and it owns whatever thread it is already on rather than taking
//! one from a runtime's blocking pool.

use super::connection::RusqliteConnection;
use super::params::to_sql_params;
use crate::blocking_trait_def::DbConnectionBlocking;
use crate::error::DbError;
use toolu_orm_core::row::{FromRow, from_rusqlite_row};
use toolu_orm_core::value::Value;

/// `execute_sql` and `query_map` reuse the connection's bounded statement
/// cache via `prepare_cached` instead of reparsing `sql` on every call. This
/// needs no invalidation logic here: SQLite revalidates a cached statement's
/// schema cookie on every execution and recompiles it against the current
/// schema before running, at the C-library level -- a schema change surfaces
/// as an ordinary query error on next use, not stale data (see
/// `rusqlite_prepared_statement_cache_test`).
impl DbConnectionBlocking for RusqliteConnection {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let guard = self.lock()?;
    // `prepare_cached` reuses this connection's bounded statement cache instead
    // of re-parsing `sql` on every call; the returned `CachedStatement` is
    // dropped (and thus returned to the cache) before the guard is released,
    // since it does not outlive this function body.
    let mut stmt = guard
      .prepare_cached(sql)
      .map_err(|e| DbError::Query(e.to_string()))?;
    let affected = stmt
      .execute(to_sql_params(&params).as_slice())
      .map_err(|e| DbError::Query(e.to_string()))?;
    Ok(affected as u64)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError> {
    let guard = self.lock()?;
    let mut stmt = guard
      .prepare_cached(sql)
      .map_err(|e| DbError::Query(e.to_string()))?;
    let mut rows = stmt
      .query(to_sql_params(&params).as_slice())
      .map_err(|e| DbError::Query(e.to_string()))?;
    let mut results = Vec::new();
    // Rows are decoded here rather than in a `query_map` callback so a `FromRow`
    // failure keeps its own message and lands in `DbError::RowMapping`, the way
    // the libsql backend reports it; routing it through a rusqlite error would
    // need a column index and type this layer does not know.
    //
    // `from_rusqlite_row` rather than a `FromRow` method: the method that exists
    // depends on the features Cargo unified onto `toolu-orm-core`, which this
    // crate's own flags do not report (issue #124).
    while let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
      results.push(from_rusqlite_row(row).map_err(DbError::from)?);
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
