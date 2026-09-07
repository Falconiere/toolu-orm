//! Tokio-postgres [`Executor`] implementation and transaction wrapper.
//!
//! # Public API
//!
//! - [`Executor`] — async execute/query for `tokio_postgres::Client`
//! - [`PgTransaction`] — transaction wrapper implementing [`Executor`]

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

use crate::QueryError;

#[async_trait::async_trait]
pub trait Executor {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError>;

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError>;
}

pub(crate) fn pg_param_slice(
  pg_params: &[Box<dyn tokio_postgres::types::ToSql + Sync + Send>],
) -> Vec<&(dyn tokio_postgres::types::ToSql + Sync)> {
  pg_params
    .iter()
    .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
    .collect()
}

async fn pg_execute(
  client: &(impl tokio_postgres::GenericClient + Sync),
  sql: &str,
  params: Vec<Value>,
) -> Result<u64, QueryError> {
  let pg_params = toolu_orm_core::value::to_pg_params(&params);
  let refs = pg_param_slice(&pg_params);
  let affected = client.execute(sql, &refs[..]).await?;
  Ok(affected)
}

async fn pg_query_map<T: FromRow + Send>(
  client: &(impl tokio_postgres::GenericClient + Sync),
  sql: &str,
  params: Vec<Value>,
) -> Result<Vec<T>, QueryError> {
  let pg_params = toolu_orm_core::value::to_pg_params(&params);
  let refs = pg_param_slice(&pg_params);
  let rows = client.query(sql, &refs[..]).await?;
  let mut results = Vec::new();
  for row in rows.iter() {
    results.push(toolu_orm_core::row::from_postgres_row(row)?);
  }
  Ok(results)
}

#[async_trait::async_trait]
impl Executor for tokio_postgres::Client {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    pg_execute(self, sql, params).await
  }

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError> {
    pg_query_map(self, sql, params).await
  }
}

/// Wrapper around `tokio_postgres::Transaction` that implements [`Executor`].
pub struct PgTransaction<'a> {
  inner: tokio_postgres::Transaction<'a>,
}

impl<'a> PgTransaction<'a> {
  pub fn new(txn: tokio_postgres::Transaction<'a>) -> Self {
    Self { inner: txn }
  }

  /// # Errors
  ///
  /// Returns [`DbCoreError::Connection`] if the commit RPC fails.
  pub async fn commit(self) -> Result<(), DbCoreError> {
    self
      .inner
      .commit()
      .await
      .map_err(|e| DbCoreError::Connection(e.to_string()))
  }

  /// # Errors
  ///
  /// Returns [`DbCoreError::Connection`] if the rollback RPC fails.
  pub async fn rollback(self) -> Result<(), DbCoreError> {
    self
      .inner
      .rollback()
      .await
      .map_err(|e| DbCoreError::Connection(e.to_string()))
  }
}

#[async_trait::async_trait]
impl<'a> Executor for PgTransaction<'a> {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    pg_execute(&self.inner, sql, params).await
  }

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError> {
    pg_query_map(&self.inner, sql, params).await
  }
}
