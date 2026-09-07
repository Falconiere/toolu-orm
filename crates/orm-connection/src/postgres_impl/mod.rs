//! Postgres backend: PgDatabase (connection pool) and PgConnection.
//!
//! Uses deadpool-postgres for connection pooling with tokio-postgres as the
//! async driver. TLS via tokio-postgres-rustls matching the project's existing
//! rustls usage.

mod connection;
mod pool;
mod tls;

pub use connection::{PgConnection, PgTransaction};
pub use pool::{PgConfig, PgDatabase};
