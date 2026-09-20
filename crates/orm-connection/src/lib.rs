//! Database connection trait and backend implementations for toolu-orm.
//!
//! ## Features
//!
//! No backend is enabled by default. Enable one or more as needed; each
//! implementation is independently feature-gated. Query-builder execution in
//! `toolu-orm-query` separately requires exactly one driver on that crate:
//!
//! - `libsql`    -- async libsql (Turso embedded replica)
//! - `rusqlite`  -- sync rusqlite (native `DbConnectionBlocking`, wrapped with
//!   spawn_blocking for `DbConnection`)
//! - `postgres`  -- async tokio-postgres + deadpool connection pool

pub mod blocking_trait_def;
pub mod error;
pub mod trait_def;

#[cfg(feature = "libsql")]
pub mod libsql_impl;

#[cfg(feature = "rusqlite")]
pub mod rusqlite_impl;

#[cfg(feature = "postgres")]
pub mod postgres_impl;

pub use blocking_trait_def::DbConnectionBlocking;
pub use error::DbError;
pub use trait_def::DbConnection;

#[cfg(feature = "libsql")]
pub use libsql_impl::{Database, LibsqlConnection, RemoteConfig};

#[cfg(feature = "rusqlite")]
pub use rusqlite_impl::{
  AttachedDatabase, IntegrityReport, MaintenanceError, RusqliteConnection, SqliteMaintenance,
  StorageStats,
};

#[cfg(feature = "postgres")]
pub use postgres_impl::{PgConfig, PgConnection, PgDatabase, PgTransaction};
