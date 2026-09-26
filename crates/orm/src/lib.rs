//! Re-exports schema, query, connection, and macro crates through one facade.
//!
//! `libsql`, `rusqlite`, `postgres`, and `lancedb` forward to all four crates.
//! Legacy query execution and fetching need exactly one implemented driver with
//! `lancedb` absent. `InsertBuilder`, `UpdateBuilder`, and `DeleteBuilder` also
//! provide async `execute_on(&impl connection::DbConnection)`, which is available
//! for every feature set and renders a write through the connection's dialect.
//! The `lancedb` feature exposes Lance extension startup, local namespace
//! lifecycle, and an attached-catalog `DbConnection` session through
//! `toolu_orm::connection`. Shared portable write execution does not fetch
//! `RETURNING` rows or add database capability checks.
//! Proc macros resolve through this facade when it is the only dependency.

pub use toolu_orm_connection as connection;
pub use toolu_orm_core as core;
pub use toolu_orm_query as query;

#[cfg(feature = "lancedb")]
pub use toolu_orm_connection::{to_duckdb_params, LanceValueError};

pub use toolu_orm_macros::{fts5_table, table, vec0_table, ColumnEnum, FromRow, Relational};

/// The macros plus the crate names they used to require.
///
/// The expansions resolve on their own now, so this is a convenience: glob it
/// when your own code wants to write `toolu_orm_core::…`, `toolu_orm_query::…`
/// or the driver crate for the enabled feature without naming the facade path
/// each time.
pub mod prelude {
  pub use toolu_orm_core;
  pub use toolu_orm_query;

  pub use toolu_orm_macros::{fts5_table, table, vec0_table, ColumnEnum, FromRow, Relational};

  #[cfg(feature = "libsql")]
  pub use toolu_orm_core::libsql;
  #[cfg(feature = "rusqlite")]
  pub use toolu_orm_core::rusqlite;
  #[cfg(feature = "postgres")]
  pub use toolu_orm_core::tokio_postgres;
}
