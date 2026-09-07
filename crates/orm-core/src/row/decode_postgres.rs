//! Postgres row decoding helper that dispatches to the correct FromRow method.

use super::FromRow;
use crate::error::DbCoreError;

/// Decodes a Postgres row using the correct [`FromRow`] method for the active `orm-core` feature set.
///
/// When `postgres` is combined with `libsql` or `rusqlite` on this crate, [`FromRow`] exposes
/// `from_pg_row` instead of `from_row`.
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when the row cannot be decoded into `T`.
#[cfg(feature = "postgres")]
pub fn from_postgres_row<T: FromRow>(row: &tokio_postgres::Row) -> Result<T, DbCoreError> {
  #[cfg(all(feature = "postgres", any(feature = "libsql", feature = "rusqlite"),))]
  {
    T::from_pg_row(row)
  }
  #[cfg(all(
    feature = "postgres",
    not(feature = "libsql"),
    not(feature = "rusqlite"),
  ))]
  {
    T::from_row(row)
  }
}
