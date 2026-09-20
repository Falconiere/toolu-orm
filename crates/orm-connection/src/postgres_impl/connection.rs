//! PgConnection and PgTransaction implementing DbConnection for Postgres.
//!
//! `execute_sql` and `query_map` prepare through deadpool's per-connection
//! `StatementCache`, shared with that connection's transactions and dropped
//! with it. Nothing bounds its size — see
//! [`crate::PgDatabase::clear_statement_caches`]. A statement the server
//! rejects is dropped from the cache and its error returned
//! unchanged, never retried, so DDL that invalidates a cached plan cannot
//! poison the connection. Statements are session state: a transaction-pooling
//! proxy must track them (PgBouncer >= 1.21) or answer `26000`.

use crate::error::DbError;
use crate::trait_def::DbConnection;
use deadpool_postgres::{GenericClient, StatementCache};
use postgres_types::ToSql;
use tokio_postgres::Statement;
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

/// A deadpool client or transaction, plus the statement cache of the
/// connection behind it.
///
/// `deadpool_postgres::GenericClient` already offers `prepare_cached`, but not
/// the cache itself, which the helpers below need to drop a rejected entry.
/// `deadpool_postgres::Transaction` holds a clone of the `Arc<StatementCache>`
/// of the client it was started on, so both impls name one cache per physical
/// connection.
trait CachedClient: GenericClient + Send {
  fn statement_cache(&self) -> &StatementCache;
}

impl CachedClient for deadpool_postgres::Client {
  fn statement_cache(&self) -> &StatementCache {
    &self.statement_cache
  }
}

impl CachedClient for deadpool_postgres::Transaction<'_> {
  fn statement_cache(&self) -> &StatementCache {
    &self.statement_cache
  }
}

/// Prepares `sql` through the connection's cache, reusing the statement when
/// the same text was already prepared on this connection.
async fn pg_prepare(client: &impl CachedClient, sql: &str) -> Result<Statement, DbError> {
  client
    .prepare_cached(sql)
    .await
    .map_err(|e| DbError::Query(pg_error_msg(&e)))
}

/// Maps a failed execution to `DbError::Query` and drops `sql` from the
/// connection's cache, so the next call re-prepares it.
///
/// Nothing is retried here: the caller sees the original error, and a
/// non-idempotent write is never replayed after an uncertain outcome.
fn pg_failed(client: &impl CachedClient, sql: &str, e: &tokio_postgres::Error) -> DbError {
  // `remove` hands back the evicted `Statement`; dropping the last handle to it
  // is what closes it on the server, and `Option` is `#[must_use]`.
  drop(client.statement_cache().remove(sql, &[]));
  DbError::Query(pg_error_msg(e))
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
  client: &impl CachedClient,
  sql: &str,
  params: Vec<Value>,
) -> Result<u64, DbError> {
  let boxed = to_pg_params(&params).map_err(|error| DbError::Query(error.to_string()))?;
  let refs = pg_param_refs(&boxed);
  let stmt = pg_prepare(client, sql).await?;
  let affected = client
    .execute(&stmt, &refs)
    .await
    .map_err(|e| pg_failed(client, sql, &e))?;
  Ok(affected)
}

async fn pg_query_map<T: FromRow + Send + 'static>(
  client: &impl CachedClient,
  sql: &str,
  params: Vec<Value>,
) -> Result<Vec<T>, DbError> {
  let boxed = to_pg_params(&params).map_err(|error| DbError::Query(error.to_string()))?;
  let refs = pg_param_refs(&boxed);
  let stmt = pg_prepare(client, sql).await?;
  let rows = client
    .query(&stmt, &refs)
    .await
    .map_err(|e| pg_failed(client, sql, &e))?;
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
