//! Rusqlite [`Executor`] implementation.
//!
//! Both methods reuse the connection's bounded statement cache via
//! `prepare_cached` instead of reparsing `sql` on every call. This needs no
//! invalidation logic here: SQLite revalidates a cached statement's schema
//! cookie on every execution and recompiles it against the current schema
//! before running, at the C-library level -- a schema change surfaces as an
//! ordinary query error on next use, not stale data (see
//! `rusqlite_prepared_statement_cache_test`).

use toolu_orm_connection::{DbConnectionBlocking, DbError, RusqliteConnection};
use toolu_orm_core::row::{from_rusqlite_row, FromRow};
use toolu_orm_core::value::Value;

use crate::QueryError;

/// Synchronous execute/query for a `rusqlite::Connection`.
pub trait Executor {
  /// # Errors
  ///
  /// Returns [`QueryError`] if the SQL statement fails.
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError>;

  /// # Errors
  ///
  /// Returns [`QueryError`] if the query or row mapping fails.
  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, QueryError>;
}

impl Executor for rusqlite::Connection {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
      .iter()
      .map(|v| v as &dyn rusqlite::types::ToSql)
      .collect();
    // `prepare_cached` reuses this connection's bounded statement cache instead
    // of re-parsing `sql` on every call; the returned `CachedStatement` is
    // dropped (and thus returned to the cache) at the end of this call.
    let mut stmt = self.prepare_cached(sql)?;
    let affected = stmt.execute(param_refs.as_slice())?;
    Ok(affected as u64)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, QueryError> {
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
      .iter()
      .map(|v| v as &dyn rusqlite::types::ToSql)
      .collect();
    let mut stmt = self.prepare_cached(sql)?;
    let mut rows = stmt.query(param_refs.as_slice())?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
      // `from_rusqlite_row` rather than a `FromRow` method: only
      // `toolu-orm-core` sees which driver features Cargo unified onto it
      // (issue #124).
      results.push(from_rusqlite_row(row)?);
    }
    Ok(results)
  }
}

impl Executor for RusqliteConnection {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    DbConnectionBlocking::execute_sql(self, sql, params).map_err(map_db)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, QueryError> {
    DbConnectionBlocking::query_map(self, sql, params).map_err(map_db)
  }
}

fn map_db(err: DbError) -> QueryError {
  match err {
    DbError::RowMapping(message) => QueryError::RowMapping {
      field: "unknown".to_owned(),
      source: toolu_orm_core::error::DbCoreError::RowMapping(message),
    },
    DbError::Connection(message) => QueryError::Connection(format!("connection: {message}")),
    DbError::Query(message) => QueryError::Connection(format!("query: {message}")),
    DbError::Transaction(message) => QueryError::Connection(format!("transaction: {message}")),
    DbError::Pool(message) => QueryError::Connection(format!("pool: {message}")),
  }
}
