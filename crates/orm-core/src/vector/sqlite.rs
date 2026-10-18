//! Strict sqlite-vec decoding for a declared vector dimension.
use super::Vector;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};

impl<const N: usize> FromSql for Vector<N> {
  fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
    Self::from_sqlite_bytes(value.as_blob()?).map_err(|error| FromSqlError::Other(Box::new(error)))
  }
}
