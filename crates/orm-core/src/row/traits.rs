//! Feature-gated FromRow trait definitions for each database backend.

#[cfg(feature = "lancedb")]
use super::LanceRow;
#[cfg(any(
  feature = "postgres",
  feature = "libsql",
  feature = "rusqlite",
  feature = "lancedb"
))]
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
/// Row decoder when Postgres is the only relational driver.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row decoder when libsql is the only relational driver.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row decoder when rusqlite is the only relational driver.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] if a column cannot be read or converted.
  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError>;
}

#[cfg(all(feature = "postgres", feature = "libsql", not(feature = "rusqlite"),))]
/// Row decoder when Postgres and libsql are enabled.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row decoder when Postgres and rusqlite are enabled.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row decoder when libsql and rusqlite are enabled.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row decoder when all relational drivers are enabled.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }

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
/// Row shape when no relational driver is enabled.
pub trait FromRow: Sized {
  const REQUIRED_COLUMNS: &'static [&'static str];

  /// Decode a portable row from the attached Lance catalog.
  /// # Errors
  /// Returns a row-mapping error when this type has no Lance decoder.
  #[cfg(feature = "lancedb")]
  fn from_lance_row(_row: &LanceRow) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "Lance FromRow decoder is not implemented for this type".into(),
    ))
  }
}
