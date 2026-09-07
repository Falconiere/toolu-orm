//! Database connection trait and backend implementations for toolu-orm.
//!
//! ## Features
//!
//! Enable exactly one backend per consumer:
//! - `libsql`    -- async libsql (Turso embedded replica)
//! - `rusqlite`  -- sync rusqlite (wrapped with spawn_blocking)
//! - `postgres`  -- async tokio-postgres + deadpool connection pool

pub mod error;
pub mod trait_def;

#[cfg(feature = "libsql")]
pub mod libsql_impl;

#[cfg(feature = "rusqlite")]
pub mod rusqlite_impl;

#[cfg(feature = "postgres")]
pub mod postgres_impl;

pub use error::DbError;
pub use trait_def::DbConnection;

#[cfg(feature = "libsql")]
pub use libsql_impl::{Database, LibsqlConnection, RemoteConfig};

#[cfg(feature = "rusqlite")]
pub use rusqlite_impl::RusqliteConnection;

#[cfg(feature = "postgres")]
pub use postgres_impl::{PgConfig, PgConnection, PgDatabase, PgTransaction};
