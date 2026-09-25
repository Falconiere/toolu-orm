//! LibSQL [`Executor`] implementation.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::row::{from_libsql_row, FromRow};
use toolu_orm_core::value::Value;

use crate::QueryError;

/// Async execute/query for a `libsql::Connection`.
#[async_trait::async_trait]
pub trait Executor {
  /// Dialect selected by this executor; override for a runtime-selected backend.
  fn dialect(&self) -> Dialect {
    Dialect::CURRENT
  }

  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError>;

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError>;
}

#[async_trait::async_trait]
impl Executor for libsql::Connection {
  fn dialect(&self) -> Dialect {
    Dialect::Sqlite
  }

  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let affected = self
      .execute(sql, libsql::params_from_iter(libsql_params))
      .await?;
    Ok(affected)
  }

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError> {
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let mut rows = self
      .query(sql, libsql::params_from_iter(libsql_params))
      .await?;
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
      // `from_libsql_row` rather than a `FromRow` method: only `toolu-orm-core`
      // sees which driver features Cargo unified onto it (issue #124).
      results.push(from_libsql_row(&row)?);
    }
    Ok(results)
  }
}
