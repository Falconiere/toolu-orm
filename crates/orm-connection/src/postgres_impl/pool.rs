//! PgDatabase and PgConfig for deadpool-postgres connection pooling.

use crate::error::DbError;

use super::connection::PgConnection;
use super::tls::make_rustls_config;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};

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
}

impl PgConfig {
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
}

impl PgDatabase {
  /// Create a new connection pool and verify connectivity.
  ///
  /// Uses `RecyclingMethod::Fast` for connection recycling and
  /// `Runtime::Tokio1` for the deadpool runtime.
  ///
  /// Performs an eager connectivity check by acquiring and releasing
  /// one connection immediately. This ensures the database is reachable
  /// at startup rather than failing on the first query.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Connection` if the pool cannot be created or
  /// the initial connectivity check fails.
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

    let pool = if config.ssl {
      let tls_config = make_rustls_config()?;
      let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);
      let mgr = Manager::from_config(pg_config, tls, mgr_config);
      Pool::builder(mgr)
        .max_size(config.max_connections)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| DbError::Pool(format!("pool creation failed: {e}")))?
    } else {
      let mgr = Manager::from_config(pg_config, tokio_postgres::NoTls, mgr_config);
      Pool::builder(mgr)
        .max_size(config.max_connections)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| DbError::Pool(format!("pool creation failed: {e}")))?
    };

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
      "postgres connection pool initialized"
    );

    Ok(Self { pool })
  }

  /// Acquire a connection from the pool.
  ///
  /// The connection is automatically returned to the pool when dropped.
  ///
  /// # Errors
  ///
  /// Returns `DbError::Pool` if no connection is available.
  pub async fn connect(&self) -> Result<PgConnection, DbError> {
    let client = self
      .pool
      .get()
      .await
      .map_err(|e| DbError::Pool(format!("pool checkout failed: {e}")))?;
    Ok(PgConnection { client })
  }
}
