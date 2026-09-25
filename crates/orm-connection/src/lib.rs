//! Connection traits and implementations for libsql, rusqlite, and Postgres.
//!
//! No driver is enabled by default. The optional `lancedb` feature opens
//! embedded DuckDB with a caller-supplied pinned Lance extension.

/// Synchronous connection trait.
pub mod blocking_trait_def;
/// Connection errors.
pub mod error;
/// Asynchronous connection trait.
pub mod trait_def;

#[cfg(feature = "libsql")]
/// Libsql connection implementation.
pub mod libsql_impl;

#[cfg(feature = "rusqlite")]
/// Rusqlite connection implementation.
pub mod rusqlite_impl;

#[cfg(feature = "postgres")]
/// PostgreSQL connection implementation.
pub mod postgres_impl;

#[cfg(feature = "lancedb")]
/// Embedded DuckDB and Lance extension startup and local namespace lifecycle.
pub mod lancedb;

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

#[cfg(feature = "lancedb")]
pub use lancedb::{
  LanceColumn, LanceColumnType, LanceConnection, LanceNamespace, LanceNamespaceError,
  LanceStartupError,
};
