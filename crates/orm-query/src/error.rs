//! QueryError enum for query execution failures.

use toolu_orm_core::error::DbCoreError;

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
  #[cfg(all(
    feature = "libsql",
    not(feature = "rusqlite"),
    not(feature = "postgres")
  ))]
  #[error("database error: {0}")]
  Driver(#[from] libsql::Error),

  #[cfg(all(
    feature = "rusqlite",
    not(feature = "libsql"),
    not(feature = "postgres")
  ))]
  #[error("database error: {0}")]
  Driver(#[from] rusqlite::Error),

  #[cfg(all(
    feature = "postgres",
    not(feature = "libsql"),
    not(feature = "rusqlite")
  ))]
  #[error("database error: {0}")]
  Driver(#[from] tokio_postgres::Error),

  #[error("row mapping error for field '{field}': {source}")]
  RowMapping { field: String, source: DbCoreError },

  #[error("no rows found in table '{table}'")]
  NotFound { table: String },

  #[error("transaction failed: {0}")]
  Transaction(Box<QueryError>),
}

impl From<DbCoreError> for QueryError {
  fn from(e: DbCoreError) -> Self {
    QueryError::RowMapping {
      field: "unknown".to_owned(),
      source: e,
    }
  }
}
