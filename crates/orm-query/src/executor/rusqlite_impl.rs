//! Rusqlite [`Executor`] implementation.

use toolu_orm_connection::{DbConnectionBlocking, DbError, RusqliteConnection};
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

use crate::QueryError;

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
      results.push(T::from_row(row)?);
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
