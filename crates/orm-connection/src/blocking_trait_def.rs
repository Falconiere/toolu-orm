//! DbConnectionBlocking trait for backends that are natively synchronous.

use crate::error::DbError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// Synchronous counterpart to [`DbConnection`](crate::trait_def::DbConnection),
/// for in-process drivers that are natively blocking.
///
/// Only `RusqliteConnection` implements it: SQLite runs in the calling process and
/// never yields, so nothing is gained by going through a runtime. libsql and
/// Postgres talk to a server and are genuinely async; a blocking wrapper there
/// would only hide a `block_on`.
///
/// Reach for this when the consumer has no async of its own -- a CLI that would
/// otherwise build a runtime per invocation -- or when it already runs its database
/// work on a blocking thread and does not want a second thread hop underneath.
///
/// `RusqliteConnection` implements this *and* `DbConnection`, and the two traits
/// share method names. With both in scope a plain `conn.execute_sql(..)` is
/// ambiguous (`E0034`); name the trait to pick a path:
///
/// ```ignore
/// DbConnectionBlocking::execute_sql(&conn, sql, params)?;
/// DbConnection::execute_sql(&conn, sql, params).await?;
/// ```
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
  /// Returns `DbError::Query` if the SQL execution fails or if a row cannot be
  /// converted into `T`.
  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError>;

  /// Execute a batch of SQL statements (e.g., DDL, migrations).
  ///
  /// # Errors
  ///
  /// Returns `DbError::Query` if any statement in the batch fails.
  fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
}
