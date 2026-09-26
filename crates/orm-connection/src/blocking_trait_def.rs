//! DbConnectionBlocking trait for backends that are natively synchronous.

use crate::error::DbError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// Synchronous SQL for the in-process rusqlite and Lance drivers.
///
/// Use it without a Tokio runtime or inside an already-blocking task. These
/// drivers also implement [`DbConnection`](crate::trait_def::DbConnection), so
/// call shared method names through the chosen trait explicitly, for example
/// `DbConnectionBlocking::execute_sql(&conn, sql, params)`.
pub trait DbConnectionBlocking: Send + Sync {
  /// Execute a write statement (INSERT/UPDATE/DELETE) and return affected row count.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if the SQL execution fails.
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError>;

  /// Execute a SELECT statement and map each row into `T` via `FromRow`.
  ///
  /// Unlike the async trait, `T` needs no `Send + 'static` bound: rows are
  /// decoded and returned on the caller's own thread.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if the SQL execution fails or `DbError::RowMapping`
  /// if a row cannot be converted into `T`.
  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError>;

  /// Execute a batch of SQL statements (e.g., DDL, migrations).
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if any statement in the batch fails.
  fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
