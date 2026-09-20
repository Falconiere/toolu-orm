//! The async surface: admit one operation at a time, then hand it to a blocking
//! thread.
//!
//! **Queue.** Every method waits for one shared permit before calling
//! `spawn_blocking`, on the caller's own task, so contending callers cost no
//! thread from tokio's finite blocking pool. Admission is FIFO; a synchronous
//! `DbConnectionBlocking` caller cannot await a permit and takes the mutex.
//!
//! **Cancellation.** The permit is *owned* and moves into the closure, so it is
//! released when the statement ends or unwinds, never when the awaiting future
//! is dropped -- a started blocking task cannot be aborted. Dropping before
//! admission runs no SQL (`docs/scenarios/rusqlite-async-backpressure.md`).

use super::connection::RusqliteConnection;
use crate::blocking_trait_def::DbConnectionBlocking;
use crate::error::DbError;
use crate::trait_def::DbConnection;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// A blocking task that never finished -- it panicked, or the runtime cancelled
/// it -- says nothing about the statement and leaves the connection in an
/// unknown state, so it is a connection failure rather than a query failure.
fn join_failure(error: &tokio::task::JoinError) -> DbError {
  DbError::Connection(format!(
    "the rusqlite blocking task did not complete: {error}"
  ))
}

/// Run one blocking statement under the connection's admission gate.
///
/// The permit is taken before `spawn_blocking` and moved into the closure, so
/// this connection's async path occupies at most one blocking thread at a time,
/// for precisely as long as the statement runs.
async fn gated<F, R>(conn: &RusqliteConnection, statement: F) -> Result<R, DbError>
where
  F: FnOnce(&RusqliteConnection) -> Result<R, DbError> + Send + 'static,
  R: Send + 'static,
{
  let permit = conn.admit().await?;
  let handle = conn.handle();
  tokio::task::spawn_blocking(move || {
    let outcome = statement(&handle);
    // Explicit, and last: the gate reopens only once the connection is free.
    // Holding the permit in the future instead would release it on
    // cancellation while this statement still owns the connection.
    drop(permit);
    outcome
  })
  .await
  .map_err(|e| join_failure(&e))?
}

/// Every method hands off to its [`DbConnectionBlocking`] twin on a blocking
/// thread, so the two surfaces run the same statement code and cannot drift.
/// Statement errors therefore arrive unchanged; only a task that never finished
/// is classified here, by `join_failure`.
#[async_trait::async_trait]
impl DbConnection for RusqliteConnection {
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
