//! LibSQL row decoding helper that dispatches to the correct FromRow method.

use super::FromRow;
use crate::error::DbCoreError;

/// Decodes a libsql row using the correct [`FromRow`] method for the active `orm-core` feature set.
///
/// When `libsql` is combined with `postgres` or `rusqlite` on this crate, [`FromRow`] exposes
/// `from_libsql_row` instead of `from_row`. Crates outside `orm-core` call this rather than
/// naming a method themselves: only this crate sees the features Cargo actually unified.
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when the row cannot be decoded into `T`.
#[cfg(feature = "libsql")]
pub fn from_libsql_row<T: FromRow>(row: &libsql::Row) -> Result<T, DbCoreError> {
  #[cfg(all(feature = "libsql", any(feature = "postgres", feature = "rusqlite"),))]
  {
    T::from_libsql_row(row)
  }
  #[cfg(all(
    feature = "libsql",
    not(feature = "postgres"),
    not(feature = "rusqlite"),
  ))]
  {
    T::from_row(row)
  }
}
