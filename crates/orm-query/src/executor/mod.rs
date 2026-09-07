//! Database command executor abstraction (libsql, rusqlite, or postgres).
//!
//! - [`Executor`] -- trait implemented by the active driver connection type
//! - [`PgTransaction`] -- postgres-only transaction wrapper

#[cfg(all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")))]
mod libsql_impl;
#[cfg(all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")))]
pub use libsql_impl::Executor;

#[cfg(all(feature = "rusqlite", not(feature = "libsql"), not(feature = "postgres")))]
mod rusqlite_impl;
#[cfg(all(feature = "rusqlite", not(feature = "libsql"), not(feature = "postgres")))]
pub use rusqlite_impl::Executor;

#[cfg(all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")))]
mod postgres;
#[cfg(all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")))]
pub use postgres::{Executor, PgTransaction};
