//! Feature-gated FromRow trait definitions for each database backend.

#[cfg(any(feature = "postgres", feature = "libsql", feature = "rusqlite"))]
use crate::error::DbCoreError;

/// Trait for converting a database row into a Rust struct.
/// Implemented via `#[derive(FromRow)]` in orm-macros or manually.
///
/// When Cargo unifies **multiple** backend features on `toolu-orm-core`, this
/// trait exposes one method per active backend (`from_pg_row`, `from_libsql_row`,
/// `from_rusqlite_row`). With exactly one backend enabled, a single
/// `from_row` method is used for that backend's row type.
#[cfg(all(
  feature = "postgres",
  not(feature = "libsql"),
  not(feature = "rusqlite"),
))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError>;
}

#[cfg(all(
  feature = "libsql",
  not(feature = "postgres"),
  not(feature = "rusqlite"),
))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_row(row: &libsql::Row) -> Result<Self, DbCoreError>;
}

#[cfg(all(
  feature = "rusqlite",
  not(feature = "postgres"),
  not(feature = "libsql"),
))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError>;
}

#[cfg(all(feature = "postgres", feature = "libsql", not(feature = "rusqlite"),))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError>;

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_libsql_row(row: &libsql::Row) -> Result<Self, DbCoreError>;
}

#[cfg(all(feature = "postgres", feature = "rusqlite", not(feature = "libsql"),))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError>;

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_rusqlite_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError>;
}

#[cfg(all(feature = "libsql", feature = "rusqlite", not(feature = "postgres"),))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_libsql_row(row: &libsql::Row) -> Result<Self, DbCoreError>;

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_rusqlite_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError>;
}

#[cfg(all(feature = "postgres", feature = "libsql", feature = "rusqlite",))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self, DbCoreError>;

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_libsql_row(row: &libsql::Row) -> Result<Self, DbCoreError>;

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_rusqlite_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError>;
}

/// Fallback when no row backend feature is enabled on this crate build.
#[cfg(not(any(
  all(
    feature = "postgres",
    not(feature = "libsql"),
    not(feature = "rusqlite")
  ),
  all(
    feature = "libsql",
    not(feature = "postgres"),
    not(feature = "rusqlite")
  ),
  all(
    feature = "rusqlite",
    not(feature = "postgres"),
    not(feature = "libsql")
  ),
  all(feature = "postgres", feature = "libsql", not(feature = "rusqlite")),
  all(feature = "postgres", feature = "rusqlite", not(feature = "libsql")),
  all(feature = "libsql", feature = "rusqlite", not(feature = "postgres")),
  all(feature = "postgres", feature = "libsql", feature = "rusqlite"),
)))]
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];
}
