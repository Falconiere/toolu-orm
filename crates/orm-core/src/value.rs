//! ORM Value enum bridging Rust types to database driver parameters.

use crate::error::DbCoreError;

/// Which Postgres JSON type a [`Value::Json`] payload binds as.
///
/// SQLite stores either one as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonStorage {
  /// Postgres `json`.
  Json,
  /// Postgres `jsonb`.
  Jsonb,
}

#[derive(Debug, Clone, PartialEq)]
/// Portable scalar value and driver binding representation.
pub enum Value {
  Null,
  Integer(i64),
  Real(f64),
  Text(String),
  Blob(Vec<u8>),
  /// Postgres `boolean`. SQLite binds `0` or `1` via [`Value::sqlite_stored`].
  Boolean(bool),
  /// Unix-epoch seconds for a Postgres `timestamptz`. SQLite binds the integer.
  TimestampEpoch(i64),
  /// RFC3339 text for a Postgres `timestamptz`. SQLite binds the text.
  TimestampText(String),
  /// JSON text. `storage` selects `json` or `jsonb` on Postgres; SQLite binds the text.
  Json {
    text: String,
    storage: JsonStorage,
  },
  /// UUID text. Postgres sends the binary uuid; SQLite binds the text.
  Uuid(String),
  /// Decimal text for Postgres `numeric`. SQLite binds the text.
  Numeric(String),
}

#[path = "value_sqlite.rs"]
mod sqlite_stored;

#[cfg(any(feature = "libsql", feature = "rusqlite"))]
use sqlite_stored::SqliteForm;

impl Value {
  /// An embedding as the little-endian `f32` bytes a `vec0` `float[N]`
  /// parameter is read from — the conversion every caller would otherwise
  /// hand-roll.
  ///
  /// `int8` and `bit` vectors are already byte slices; pass those as
  /// [`Value::Blob`].
  ///
  /// ```
  /// use toolu_orm_core::value::Value;
  ///
  /// assert_eq!(Value::vector(&[1.0f32]), Value::Blob(vec![0, 0, 128, 63]));
  /// ```
  #[must_use]
  pub fn vector(embedding: &[f32]) -> Self {
    let mut bytes = Vec::with_capacity(size_of_val(embedding));
    for element in embedding {
      bytes.extend_from_slice(&element.to_le_bytes());
    }
    Value::Blob(bytes)
  }

  /// [`Self::vector`], refusing a slice that is not `dim` long.
  ///
  /// The declared dimension is known at the call site; `vec0` only discovers
  /// the mismatch at insert time and reports it as a byte count.
  ///
  /// # Errors
  ///
  /// [`DbCoreError::VectorDimension`] when the slice length is not `dim`.
  pub fn vector_with_dim(embedding: &[f32], dim: u32) -> Result<Self, DbCoreError> {
    if embedding.len() != dim as usize {
      return Err(DbCoreError::VectorDimension {
        expected: dim,
        actual: embedding.len(),
      });
    }
    Ok(Self::vector(embedding))
  }
}

impl From<&str> for Value {
  fn from(s: &str) -> Self {
    Value::Text(s.to_owned())
  }
}

impl From<String> for Value {
  fn from(s: String) -> Self {
    Value::Text(s)
  }
}

impl From<i32> for Value {
  fn from(n: i32) -> Self {
    Value::Integer(i64::from(n))
  }
}

impl From<i64> for Value {
  fn from(n: i64) -> Self {
    Value::Integer(n)
  }
}

impl From<f64> for Value {
  fn from(f: f64) -> Self {
    Value::Real(f)
  }
}

impl From<bool> for Value {
  fn from(b: bool) -> Self {
    Value::Integer(if b { 1 } else { 0 })
  }
}

impl From<Vec<u8>> for Value {
  fn from(b: Vec<u8>) -> Self {
    Value::Blob(b)
  }
}

impl<T: Into<Value>> From<Option<T>> for Value {
  fn from(opt: Option<T>) -> Self {
    match opt {
      Some(v) => v.into(),
      None => Value::Null,
    }
  }
}

#[cfg(feature = "libsql")]
impl From<Value> for libsql::Value {
  fn from(v: Value) -> Self {
    match v.sqlite_form() {
      SqliteForm::Null => libsql::Value::Null,
      SqliteForm::Integer(n) => libsql::Value::Integer(n),
      SqliteForm::Real(f) => libsql::Value::Real(f),
      SqliteForm::Text(s) => libsql::Value::Text(s.to_owned()),
      SqliteForm::Blob(b) => libsql::Value::Blob(b.to_owned()),
    }
  }
}

#[cfg(feature = "libsql")]
impl From<libsql::Value> for Value {
  fn from(v: libsql::Value) -> Self {
    match v {
      libsql::Value::Null => Value::Null,
      libsql::Value::Integer(n) => Value::Integer(n),
      libsql::Value::Real(f) => Value::Real(f),
      libsql::Value::Text(s) => Value::Text(s),
      libsql::Value::Blob(b) => Value::Blob(b),
    }
  }
}

#[cfg(feature = "rusqlite")]
impl rusqlite::types::ToSql for Value {
  fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
    match self.sqlite_form() {
      SqliteForm::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Null,
      )),
      SqliteForm::Integer(n) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Integer(n),
      )),
      SqliteForm::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Real(f),
      )),
      SqliteForm::Text(s) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Text(s.to_owned()),
      )),
      SqliteForm::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Blob(b.to_owned()),
      )),
    }
  }
}

#[cfg(feature = "postgres")]
#[path = "value_pg.rs"]
mod pg_conversions;

#[cfg(feature = "postgres")]
pub use pg_conversions::to_pg_params;
