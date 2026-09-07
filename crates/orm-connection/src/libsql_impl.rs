//! libsql backend: Database factory and LibsqlConnection.

use std::sync::Arc;

use crate::error::DbError;
use crate::trait_def::DbConnection;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

/// Configuration for synced remote replica.
pub struct RemoteConfig {
  /// Path to the local replica file.
  pub replica_path: String,
  /// Remote database URL.
  pub url: String,
  /// Authentication token for the remote database.
  pub auth_token: String,
  /// Sync interval in seconds (default: 5).
  pub sync_interval_secs: u64,
  /// Maximum retry attempts for initial sync (default: 5).
  pub max_sync_attempts: u32,
}

/// Wraps `Arc<libsql::Database>` with init helpers.
#[derive(Clone)]
pub struct Database {
  inner: Arc<libsql::Database>,
}

impl Database {
  /// Open a local-only SQLite file. Strips `file://` or `file:` prefix if present.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the database cannot be opened.
  pub async fn init_local(path: &str) -> Result<Self, DbError> {
    let clean_path = path
      .strip_prefix("file://")
      .or_else(|| path.strip_prefix("file:"))
      .unwrap_or(path);
    let db = libsql::Builder::new_local(clean_path)
      .build()
      .await
      .map_err(|e| DbError::Connection(e.to_string()))?;
    Ok(Self {
      inner: Arc::new(db),
    })
  }

  /// Build a synced embedded replica with retry/rebuild on stale frames.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the database cannot be opened or
  /// the initial sync fails after all retry attempts.
  pub async fn init_remote(config: RemoteConfig) -> Result<Self, DbError> {
    clean_inconsistent_replica(&config.replica_path);
    let mut db = build_synced_db(&config).await?;

    for attempt in 1..=config.max_sync_attempts {
      let err_msg = match db.sync().await {
        Ok(_) => {
          tracing::info!("initial database sync complete");
          return Ok(Self {
            inner: Arc::new(db),
          });
        },
        Err(e) => e.to_string(),
      };

      db = handle_sync_failure(db, &err_msg, attempt, config.max_sync_attempts, &config).await?;
    }

    Err(DbError::Connection(format!(
      "initial sync failed: replica conflict persists after {} rebuilds",
      config.max_sync_attempts
    )))
  }

  /// Get a reference to the inner `Arc<libsql::Database>`.
  pub fn inner(&self) -> &Arc<libsql::Database> {
    &self.inner
  }

  /// Create a new connection to the database.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the connection cannot be established.
  pub fn connect(&self) -> Result<LibsqlConnection, DbError> {
    let conn = self
      .inner
      .connect()
      .map_err(|e| DbError::Connection(format!("connection failed: {e}")))?;
    Ok(LibsqlConnection { inner: conn })
  }
}

/// Wraps `libsql::Connection` and implements [`DbConnection`].
pub struct LibsqlConnection {
  inner: libsql::Connection,
}

impl LibsqlConnection {
  /// Wrap an existing libSQL connection (for example from [`Database::connect`]).
  #[must_use]
  pub fn new(inner: libsql::Connection) -> Self {
    Self { inner }
  }

  /// Access the underlying `libsql::Connection` for raw SQL queries
  /// that cannot use the [`DbConnection`] trait (e.g., multi-column SELECT
  /// with manual row parsing).
  pub fn inner_conn(&self) -> &libsql::Connection {
    &self.inner
  }
}

#[async_trait::async_trait]
impl DbConnection for LibsqlConnection {
  async fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, DbError> {
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let affected = self
      .inner
      .execute(sql, libsql::params_from_iter(libsql_params))
      .await
      .map_err(|e| DbError::Query(e.to_string()))?;
    Ok(affected)
  }

  async fn query_map<T: FromRow + Send + 'static>(
    &self,
    sql: &str,
    params: Vec<Value>,
  ) -> Result<Vec<T>, DbError> {
    let libsql_params: Vec<libsql::Value> = params.into_iter().map(Into::into).collect();
    let mut rows = self
      .inner
      .query(sql, libsql::params_from_iter(libsql_params))
      .await
      .map_err(|e| DbError::Query(e.to_string()))?;
    let mut results = Vec::new();
    while let Some(row) = rows
      .next()
      .await
      .map_err(|e| DbError::Query(e.to_string()))?
    {
      #[cfg(all(feature = "libsql", any(feature = "postgres", feature = "rusqlite"),))]
      results.push(T::from_libsql_row(&row).map_err(DbError::from)?);
      #[cfg(all(
        feature = "libsql",
        not(feature = "postgres"),
        not(feature = "rusqlite"),
      ))]
      results.push(T::from_row(&row).map_err(DbError::from)?);
    }
    Ok(results)
  }

  async fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
    self
      .inner
      .execute_batch(sql)
      .await
      .map_err(|e| DbError::Query(e.to_string()))?;
    Ok(())
  }
}

async fn build_synced_db(config: &RemoteConfig) -> Result<libsql::Database, DbError> {
  libsql::Builder::new_synced_database(
    &config.replica_path,
    config.url.clone(),
    config.auth_token.clone(),
  )
  .sync_interval(std::time::Duration::from_secs(config.sync_interval_secs))
  .build()
  .await
  .map_err(|e| DbError::Connection(e.to_string()))
}

async fn handle_sync_failure(
  db: libsql::Database,
  err_msg: &str,
  attempt: u32,
  max_attempts: u32,
  config: &RemoteConfig,
) -> Result<libsql::Database, DbError> {
  if err_msg.contains("conflict") || err_msg.contains("Generation ID mismatch") {
    return rebuild_stale_replica(db, attempt, max_attempts, config).await;
  }
  retry_sync_or_fail(err_msg, attempt, max_attempts).await?;
  Ok(db)
}

async fn rebuild_stale_replica(
  db: libsql::Database,
  attempt: u32,
  max_attempts: u32,
  config: &RemoteConfig,
) -> Result<libsql::Database, DbError> {
  tracing::warn!("stale replica detected (attempt {attempt}/{max_attempts}), rebuilding");
  drop(db);
  remove_replica_files(&config.replica_path);
  build_synced_db(config).await
}

async fn retry_sync_or_fail(err_msg: &str, attempt: u32, max_attempts: u32) -> Result<(), DbError> {
  if attempt >= max_attempts {
    return Err(DbError::Connection(format!(
      "initial sync failed after {max_attempts} attempts: {err_msg}"
    )));
  }
  let backoff_ms = 500u64 * 2u64.pow(attempt - 1);
  tracing::warn!(
    "sync attempt {attempt}/{max_attempts} failed: {err_msg}, retrying in {backoff_ms}ms"
  );
  tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
  Ok(())
}

fn clean_inconsistent_replica(replica_path: &str) {
  let db_exists = std::path::Path::new(replica_path).exists();
  let meta_exists = std::path::Path::new(&format!("{replica_path}-info")).exists();
  if meta_exists && !db_exists {
    tracing::warn!("inconsistent replica state detected, cleaning up stale files");
    remove_replica_files(replica_path);
  }
}

fn remove_replica_files(replica_path: &str) {
  for suffix in ["", "-wal", "-shm", "-journal", "-info"] {
    let path = format!("{replica_path}{suffix}");
    if std::fs::remove_file(&path).is_ok() {
      tracing::info!("removed stale replica file: {path}");
    }
  }
}
