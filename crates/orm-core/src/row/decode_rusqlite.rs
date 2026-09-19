//! Rusqlite row decoding helper that dispatches to the correct FromRow method.

use super::FromRow;
use crate::error::DbCoreError;

/// Decodes a rusqlite row using the correct [`FromRow`] method for the active `orm-core` feature set.
///
/// When `rusqlite` is combined with `postgres` or `libsql` on this crate, [`FromRow`] exposes
/// `from_rusqlite_row` instead of `from_row`. Crates outside `orm-core` call this rather than
/// naming a method themselves: only this crate sees the features Cargo actually unified.
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when the row cannot be decoded into `T`.
#[cfg(feature = "rusqlite")]
pub fn from_rusqlite_row<T: FromRow>(row: &rusqlite::Row<'_>) -> Result<T, DbCoreError> {
  // The two arms partition `feature = "rusqlite"` exactly -- and this function
  // exists only under it, as does the module. `any(postgres, libsql)` and its
  // negation leave no gap and no overlap, so a third arm would be unreachable.
  #[cfg(all(feature = "rusqlite", any(feature = "postgres", feature = "libsql"),))]
  {
    T::from_rusqlite_row(row)
  }
  #[cfg(all(
    feature = "rusqlite",
    not(feature = "postgres"),
    not(feature = "libsql"),
  ))]
  {
    T::from_row(row)
  }
}
