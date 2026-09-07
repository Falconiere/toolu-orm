//! Rusqlite [`Executor`] implementation.

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
    let affected = self.execute(sql, param_refs.as_slice())?;
    Ok(affected as u64)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, QueryError> {
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
      .iter()
      .map(|v| v as &dyn rusqlite::types::ToSql)
      .collect();
    let mut stmt = self.prepare(sql)?;
    let mut rows = stmt.query(param_refs.as_slice())?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
      results.push(T::from_row(row)?);
    }
    Ok(results)
  }
}
