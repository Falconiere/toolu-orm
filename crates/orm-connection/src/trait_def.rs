//! DbConnection trait shared across all database backends.

use crate::error::DbError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// Trait abstracting over database connections for testability and driver swaps.
///
/// Query functions accept `&(impl DbConnection)` so callers can provide
/// any backend (libsql, rusqlite, postgres, Lance) or a test double.
///
/// All methods are async. Sync backends (rusqlite and Lance) use `spawn_blocking`
/// internally to satisfy the async interface.
#[async_trait::async_trait]
pub trait DbConnection: Send + Sync {
  /// SQL dialect selected by this connection, independent of other enabled drivers.
  ///
  /// Built-in connections override this. Custom connections should override it
  /// for runtime selection; the default preserves the compile-time behavior of
  /// existing implementations.
  fn dialect(&self) -> toolu_orm_core::dialect::Dialect {
    toolu_orm_core::dialect::Dialect::CURRENT
  }

  /// Execute a write statement (INSERT/UPDATE/DELETE) and return affected row count.
  ///
  /// Parameters are passed as `Vec<Value>` and converted to the driver's
  /// native parameter type by each backend implementation.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if the SQL execution fails.
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError>;

  /// Execute a SELECT statement and map each row into `T` via `FromRow`.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if the SQL execution fails or `DbError::RowMapping`
  /// if a row cannot be converted into `T`.
  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError>;

  /// Execute a batch of SQL statements (e.g., DDL, migrations).
  ///
  /// The batch is executed as a single string. Backends that require
  /// statement-level execution must split internally.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if any statement in the batch fails.
  async fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
