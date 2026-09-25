//! Re-exports schema, query, connection, and macro crates through one facade.
//!
//! `libsql`, `rusqlite`, `postgres`, and `lancedb` forward to all four crates.
//! Query execution needs exactly one implemented driver with `lancedb` absent.
//! The `lancedb` feature exposes Lance extension startup through
//! `toolu_orm::connection::LanceConnection`, but no executor yet.
//! Proc macros resolve through this facade when it is the only dependency.

pub use toolu_orm_connection as connection;
pub use toolu_orm_core as core;
pub use toolu_orm_query as query;

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
