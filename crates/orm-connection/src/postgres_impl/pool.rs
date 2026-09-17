//! PgDatabase and PgConfig for deadpool-postgres connection pooling.

use std::time::Duration;

use crate::error::DbError;

use super::connection::PgConnection;
use super::tls::make_rustls_config;

use deadpool_postgres::{
  Manager, ManagerConfig, Pool, PoolError, RecyclingMethod, Runtime, TimeoutType,
};

/// Configuration for connecting to a PostgreSQL database.
#[derive(Debug, Clone)]
pub struct PgConfig {
  /// Database server hostname.
  pub host: String,
  /// Database server port (default: 5432).
  pub port: u16,
  /// Database user.
  pub user: String,
  /// Database password.
  pub password: String,
  /// Database name.
  pub dbname: String,
  /// Maximum number of connections in the pool.
  pub max_connections: usize,
  /// Require TLS for the connection (default: false).
  pub ssl: bool,
  /// Maximum time `connect()` waits for a pool slot (deadpool's checkout
  /// **wait** timeout) — distinct from connection-creation/recycle
  /// timeouts, which this config does not expose. `None` opts out for an
  /// unbounded wait. See [`PgConfig::DEFAULT_CHECKOUT_TIMEOUT`].
  pub checkout_timeout: Option<Duration>,
}

impl PgConfig {
  /// Default checkout-wait timeout used by [`PgConfig::for_test`].
  ///
  /// Bounds only the wait for an available pool slot, not connection
  /// creation or recycling. Five seconds is long enough to absorb ordinary
  /// contention while still failing fast instead of hanging indefinitely.
  /// A caller that needs an unbounded wait sets `checkout_timeout: None`
  /// explicitly.
  pub const DEFAULT_CHECKOUT_TIMEOUT: Duration = Duration::from_secs(5);

  /// Config for test databases. Defaults: host=localhost, port=5433,
  /// user/password=toolu.
  /// Override any value via `TEST_DB_HOST`, `TEST_DB_PORT`, `TEST_DB_USER`, `TEST_DB_PASSWORD`.
  #[must_use]
  pub fn for_test(dbname: impl Into<String>) -> Self {
    Self {
      host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".into()),
      port: std::env::var("TEST_DB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5433),
      user: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "toolu".into()),
      password: std::env::var("TEST_DB_PASSWORD").unwrap_or_else(|_| "toolu".into()),
      dbname: dbname.into(),
      max_connections: 5,
      ssl: false,
      checkout_timeout: Some(Self::DEFAULT_CHECKOUT_TIMEOUT),
    }
  }
}

/// PostgreSQL connection pool backed by deadpool-postgres.
///
/// Call `init()` to create the pool with an eager connectivity check,
/// then `connect()` to obtain a pooled `PgConnection`.
#[derive(Clone)]
pub struct PgDatabase {
  pool: Pool,
  checkout_timeout: Option<Duration>,
}

/// Builds the deadpool `Pool`, choosing the TLS or plaintext manager. Both
/// branches apply the same `max_size` and checkout `wait_timeout`.
fn build_pool(
  pg_config: tokio_postgres::Config,
  mgr_config: ManagerConfig,
  config: &PgConfig,
) -> Result<Pool, DbError> {
  if config.ssl {
    let tls_config = make_rustls_config()?;
    let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
    let mgr = Manager::from_config(pg_config, tls, mgr_config);
    Pool::builder(mgr)
      .max_size(config.max_connections)
      .wait_timeout(config.checkout_timeout)
      .runtime(Runtime::Tokio1)
      .build()
      .map_err(|e| DbError::Pool(format!("pool creation failed: {e}")))
  } else {
    let mgr = Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
    Pool::builder(mgr)
      .max_size(config.max_connections)
      .wait_timeout(config.checkout_timeout)
      .runtime(Runtime::Tokio1)
      .build()
      .map_err(|e| DbError::Pool(format!("pool creation failed: {e}")))
  }
}

impl PgDatabase {
  /// Creates the pool (`RecyclingMethod::Fast`, `Runtime::Tokio1`) and
  /// verifies connectivity with an eager checkout, so a bad config fails
  /// at startup rather than on the first query.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the pool cannot be created or the
  /// initial connectivity check fails.
  pub async fn init(config: &PgConfig) -> Result<Self, DbError> {
    let mut pg_config = tokio_postgres::Config::new();
    pg_config
      .host(&config.host)
      .port(config.port)
      .user(&config.user)
      .password(&config.password)
      .dbname(&config.dbname);

    let mgr_config = ManagerConfig {
      recycling_method: RecyclingMethod::Fast,
    };

    let pool = build_pool(pg_config, mgr_config, config)?;

    let _client = pool
      .get()
      .await
      .map_err(|e| DbError::Connection(format!("initial connection failed: {e}")))?;

    tracing::info!(
      host = %config.host,
      port = config.port,
      dbname = %config.dbname,
      ssl = config.ssl,
      max_connections = config.max_connections,
      checkout_timeout = ?config.checkout_timeout,
      "postgres connection pool initialized"
    );

    Ok(Self {
      pool,
      checkout_timeout: config.checkout_timeout,
    })
  }

  /// Acquire a connection from the pool.
  ///
  /// The connection is automatically returned to the pool when dropped.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Pool` if no connection is available, naming the
  /// configured checkout-wait timeout when that is why.
  pub async fn connect(&self) -> Result<PgConnection, DbError> {
    let client = self.pool.get().await.map_err(|e| self.checkout_error(&e))?;
    Ok(PgConnection { client })
  }

  fn checkout_error(&self, e: &PoolError) -> DbError {
    let PoolError::Timeout(timeout_type) = e else {
      return DbError::Pool(format!("pool checkout failed: {e}"));
    };
    if !matches!(timeout_type, TimeoutType::Wait) {
      // This config only sets deadpool's wait timeout, so Create/Recycle
      // should never fire — but if deadpool ever changes that, still label
      // it a timeout instead of falling through to the generic message.
      return DbError::Pool(format!("pool checkout timed out ({timeout_type:?}): {e}"));
    }
    match self.checkout_timeout {
      Some(d) => DbError::Pool(format!(
        "pool checkout timed out after {d:?} waiting for a connection"
      )),
      None => DbError::Pool(format!(
        "pool checkout timed out waiting for a connection: {e}"
      )),
    }
  }
}
