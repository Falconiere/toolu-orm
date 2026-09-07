//! Database-backed fetch methods for [`super::SelectBuilder`].
//!
//! Driver-specific `SelectBuilder` methods: `fetch_all`, `fetch_one`, `fetch_optional`,
//! `count`, `exists`.

mod shared;

#[cfg(all(feature = "libsql", not(feature = "rusqlite"), not(feature = "postgres")))]
mod fetch_libsql;

#[cfg(all(feature = "rusqlite", not(feature = "libsql"), not(feature = "postgres")))]
mod fetch_rusqlite;

#[cfg(all(feature = "postgres", not(feature = "libsql"), not(feature = "rusqlite")))]
mod fetch_postgres;
