//! PgConnection and PgTransaction implementing DbConnection for Postgres.

use crate::error::DbError;
use crate::trait_def::DbConnection;
use deadpool_postgres::GenericClient;
use postgres_types::ToSql;
use toolu_orm_core::row::{FromRow, from_postgres_row};
use toolu_orm_core::value::{Value, to_pg_params};

/// A pooled Postgres connection. Implements `DbConnection`.
///
/// Returned to the pool automatically on drop.
pub struct PgConnection {
  pub(super) client: deadpool_postgres::Client,
}

impl PgConnection {
  /// Begin a new transaction.
  ///
  /// The returned `PgTransaction` implements `DbConnection` so all
  /// existing query functions work inside a transaction unchanged.
  ///
  /// If `PgTransaction` is dropped without calling `commit()`, the
  /// transaction is automatically rolled back (tokio-postgres built-in).
  ///
  /// # Errors
  ///
  /// Returns `DbError::Transaction` if the BEGIN statement fails.
  pub async fn transaction(&mut self) -> Result<PgTransaction<'_>, DbError> {
    let txn = self
      .client
      .transaction()
      .await
      .map_err(|e| DbError::Transaction(format!("BEGIN failed: {e}")))?;
    Ok(PgTransaction { inner: txn })
  }
}

/// A Postgres transaction. Implements `DbConnection` for query execution.
///
/// Drop without `commit()` = automatic rollback.
pub struct PgTransaction<'a> {
  inner: deadpool_postgres::Transaction<'a>,
}

impl PgTransaction<'_> {
  /// Commit the transaction.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Transaction` if the COMMIT statement fails.
  pub async fn commit(self) -> Result<(), DbError> {
    self
      .inner
      .commit()
      .await
      .map_err(|e| DbError::Transaction(format!("COMMIT failed: {e}")))?;
    Ok(())
  }
}

fn pg_param_refs(boxed: &[Box<dyn ToSql + Sync + Send>]) -> Vec<&(dyn ToSql + Sync)> {
  boxed
    .iter()
    .map(|b| b.as_ref() as &(dyn ToSql + Sync))
    .collect()
}

fn pg_error_msg(e: &tokio_postgres::Error) -> String {
  if let Some(db_err) = e.as_db_error() {
    format!(
      "{}: {} ({})",
      db_err.severity(),
      db_err.message(),
      db_err.code().code()
    )
  } else {
    format!("{e:?}")
  }
}

async fn pg_execute_sql(
  client: &(impl GenericClient + Send),
  sql: &str,
  params: Vec<Value>,
) -> Result<u64, DbError> {
  let boxed = to_pg_params(&params);
  let refs = pg_param_refs(&boxed);
  let affected = client
    .execute(sql, &refs)
    .await
    .map_err(|e| DbError::Query(pg_error_msg(&e)))?;
  Ok(affected)
}

async fn pg_query_map<T: FromRow + Send + 'static>(
  client: &(impl GenericClient + Send),
  sql: &str,
  params: Vec<Value>,
) -> Result<Vec<T>, DbError> {
  let boxed = to_pg_params(&params);
  let refs = pg_param_refs(&boxed);
  let rows = client
    .query(sql, &refs)
    .await
    .map_err(|e| DbError::Query(pg_error_msg(&e)))?;
  let mut results = Vec::with_capacity(rows.len());
  for row in &rows {
    results.push(from_postgres_row(row).map_err(DbError::from)?);
  }
  Ok(results)
}

async fn pg_execute_batch(client: &(impl GenericClient + Send), sql: &str) -> Result<(), DbError> {
  client
    .batch_execute(sql)
    .await
    .map_err(|e| DbError::Query(e.to_string()))?;
  Ok(())
}

#[async_trait::async_trait]
impl DbConnection for PgConnection {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    pg_execute_sql(&self.client, sql, params).await
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    pg_query_map(&self.client, sql, params).await
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    pg_execute_batch(&self.client, sql).await
  }
}

#[async_trait::async_trait]
impl DbConnection for PgTransaction<'_> {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    pg_execute_sql(&self.inner, sql, params).await
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    pg_query_map(&self.inner, sql, params).await
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    pg_execute_batch(&self.inner, sql).await
  }
}
