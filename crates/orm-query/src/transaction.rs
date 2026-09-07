//! Transaction wrapper with commit and rollback support.

// ── Transaction wrapper (libsql — async) ─────────────────────────────────────

/// Wraps a `libsql::Transaction` and implements `Executor`.
///
/// The inner transaction is held behind an `Arc<tokio::sync::Mutex>` so that
/// `TransactionExt::run_transaction` can reclaim it after the user's closure
/// completes and call `commit` or let it drop (rollback).
#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
pub struct Transaction {
  inner: std::sync::Arc<tokio::sync::Mutex<Option<libsql::Transaction>>>,
}

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
use crate::QueryError;
#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
use toolu_orm_core::row::FromRow;
#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
use toolu_orm_core::value::Value;

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
impl Transaction {
  fn consumed_error() -> QueryError {
    QueryError::Transaction(Box::new(QueryError::NotFound {
      table: "transaction already consumed".to_owned(),
    }))
  }
}

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
#[async_trait::async_trait]
impl crate::executor::Executor for Transaction {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    let guard = self.inner.lock().await;
    let tx = guard.as_ref().ok_or_else(Self::consumed_error)?;
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let affected = tx
      .execute(sql, libsql::params_from_iter(libsql_params))
      .await?;
    Ok(affected)
  }

  async fn query_map<T: FromRow + Send>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, QueryError> {
    let guard = self.inner.lock().await;
    let tx = guard.as_ref().ok_or_else(Self::consumed_error)?;
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let mut rows = tx
      .query(sql, libsql::params_from_iter(libsql_params))
      .await?;
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
      results.push(T::from_row(&row)?);
    }
    Ok(results)
  }
}

// ── TransactionExt for libsql::Connection ────────────────────────────────────

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
#[async_trait::async_trait]
pub trait TransactionExt {
  /// Run a closure inside a transaction.
  ///
  /// On `Ok`, the transaction is committed. On `Err`, it is rolled back
  /// automatically when dropped.
  ///
  /// # Errors
  ///
  /// Returns `QueryError` from the closure or from `commit`.
  async fn run_transaction<F, Fut, T>(&self, f: F) -> Result<T, QueryError>
  where
    F: FnOnce(Transaction) -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, QueryError>> + Send,
    T: Send;
}

#[cfg(all(
  feature = "libsql",
  not(feature = "rusqlite"),
  not(feature = "postgres")
))]
#[async_trait::async_trait]
impl TransactionExt for libsql::Connection {
  async fn run_transaction<F, Fut, T>(&self, f: F) -> Result<T, QueryError>
  where
    F: FnOnce(Transaction) -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, QueryError>> + Send,
    T: Send,
  {
    let tx = self.transaction().await?;
    let shared = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));
    let wrapper = Transaction {
      inner: std::sync::Arc::clone(&shared),
    };
    let result = f(wrapper).await;
    match result {
      Ok(value) => {
        if let Some(tx) = shared.lock().await.take() {
          tx.commit().await?;
        }
        Ok(value)
      },
      Err(e) => {
        // Take and drop the transaction — libsql rolls back on drop.
        drop(shared.lock().await.take());
        Err(e)
      },
    }
  }
}
