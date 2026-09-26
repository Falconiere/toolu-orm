//! Serialized blocking and async SQL on an already attached Lance catalog.

use std::sync::{Arc, Mutex, MutexGuard};

use duckdb::Connection;
use toolu_orm_core::{row::FromRow, value::Value};

use super::{namespace::LanceNamespace, session_row::portable_row, to_duckdb_params};
use crate::{blocking_trait_def::DbConnectionBlocking, error::DbError, trait_def::DbConnection};

/// A selected DuckDB–Lance connection usable through the shared SQL traits.
///
/// Consume a namespace after attachment with [`Self::from_namespace`]. The
/// underlying DuckDB connection is `Send` but not `Sync`; the mutex permits
/// exactly one statement at a time. Async callers await one admission permit
/// before using Tokio's blocking pool. Dropping the last handle closes it.
pub struct LanceDbConnection {
  inner: Arc<Mutex<Connection>>,
  admission: Arc<tokio::sync::Semaphore>,
}

impl LanceDbConnection {
  /// Take ownership of the already loaded, attached, and selected namespace.
  #[must_use]
  pub fn from_namespace(namespace: LanceNamespace) -> Self {
    Self {
      inner: Arc::new(Mutex::new(namespace.into_connection())),
      admission: Arc::new(tokio::sync::Semaphore::new(1)),
    }
  }

  fn handle(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
      admission: Arc::clone(&self.admission),
    }
  }

  fn lock(&self) -> Result<MutexGuard<'_, Connection>, DbError> {
    self.inner.lock().map_err(|error| {
      DbError::Connection(format!(
        "the Lance connection lock is poisoned by an earlier panic: {error}"
      ))
    })
  }

  async fn admit(&self) -> Result<tokio::sync::OwnedSemaphorePermit, DbError> {
    Arc::clone(&self.admission)
      .acquire_owned()
      .await
      .map_err(|error| DbError::Connection(format!("Lance admission gate closed: {error}")))
  }
}

impl DbConnectionBlocking for LanceDbConnection {
  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let native = to_duckdb_params(&params).map_err(|error| DbError::Query(error.to_string()))?;
    let guard = self.lock()?;
    let mut statement = guard
      .prepare(sql)
      .map_err(|error| DbError::Query(error.to_string()))?;
    let affected = statement
      .execute(duckdb::params_from_iter(native.iter()))
      .map_err(|error| DbError::Query(error.to_string()))?;
    Ok(affected as u64)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, DbError> {
    let native = to_duckdb_params(&params).map_err(|error| DbError::Query(error.to_string()))?;
    let guard = self.lock()?;
    let mut statement = guard
      .prepare(sql)
      .map_err(|error| DbError::Query(error.to_string()))?;
    let mut rows = statement
      .query(duckdb::params_from_iter(native.iter()))
      .map_err(|error| DbError::Query(error.to_string()))?;
    let names = rows
      .as_ref()
      .ok_or_else(|| DbError::Query("Lance query returned no statement metadata".into()))?
      .column_names();
    for required in T::REQUIRED_COLUMNS {
      if !names.iter().any(|name| name.eq_ignore_ascii_case(required)) {
        return Err(DbError::RowMapping(format!(
          "missing Lance result column {required}"
        )));
      }
    }
    let mut results = Vec::new();
    while let Some(row) = rows
      .next()
      .map_err(|error| DbError::Query(error.to_string()))?
    {
      let portable = portable_row(&names, row)?;
      results.push(T::from_lance_row(&portable).map_err(DbError::from)?);
    }
    Ok(results)
  }

  fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    self
      .lock()?
      .execute_batch(sql)
      .map_err(|error| DbError::Query(format!("Lance batch {sql:?}: {error}")))
  }
}

async fn gated<F, R>(conn: &LanceDbConnection, operation: F) -> Result<R, DbError>
where
  F: FnOnce(&LanceDbConnection) -> Result<R, DbError> + Send + 'static,
  R: Send + 'static,
{
  let permit = conn.admit().await?;
  let handle = conn.handle();
  tokio::task::spawn_blocking(move || {
    let result = operation(&handle);
    drop(permit);
    result
  })
  .await
  .map_err(|error| DbError::Connection(format!("Lance blocking task did not complete: {error}")))?
}

#[async_trait::async_trait]
impl DbConnection for LanceDbConnection {
  fn dialect(&self) -> toolu_orm_core::dialect::Dialect {
    toolu_orm_core::dialect::Dialect::Lance
  }

  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let sql = sql.to_owned();
    gated(self, move |conn| {
      DbConnectionBlocking::execute_sql(conn, &sql, params)
    })
    .await
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    let sql = sql.to_owned();
    gated(self, move |conn| {
      DbConnectionBlocking::query_map(conn, &sql, params)
    })
    .await
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    let sql = sql.to_owned();
    gated(self, move |conn| {
      DbConnectionBlocking::execute_batch(conn, &sql)
    })
    .await
  }
}
