//! ORM Value enum bridging Rust types to database driver parameters.

use crate::error::DbCoreError;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
  Null,
  Integer(i64),
  Real(f64),
  Text(String),
  Blob(Vec<u8>),
}

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
    match v {
      Value::Null => libsql::Value::Null,
      Value::Integer(n) => libsql::Value::Integer(n),
      Value::Real(f) => libsql::Value::Real(f),
      Value::Text(s) => libsql::Value::Text(s),
      Value::Blob(b) => libsql::Value::Blob(b),
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
    match self {
      Value::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Null,
      )),
      Value::Integer(n) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Integer(*n),
      )),
      Value::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Real(*f),
      )),
      Value::Text(s) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Text(s.clone()),
      )),
      Value::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Owned(
        rusqlite::types::Value::Blob(b.clone()),
      )),
    }
  }
}

#[cfg(feature = "postgres")]
mod pg_conversions {
  use postgres_types::ToSql;

  use super::Value;

  /// Type-agnostic SQL NULL accepted by any Postgres column type.
  #[derive(Debug)]
  struct PgNull;

  impl postgres_types::ToSql for PgNull {
    fn to_sql(
      &self,
      _ty: &postgres_types::Type,
      _out: &mut bytes::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
      Ok(postgres_types::IsNull::Yes)
    }

    fn accepts(_ty: &postgres_types::Type) -> bool {
      true
    }

    postgres_types::to_sql_checked!();
  }

  /// Text value that also accepts Postgres UUID columns.
  ///
  /// When the target column is UUID, parses the string and writes binary format.
  /// For all other text-compatible types, delegates to the standard `String` impl.
  #[derive(Debug)]
  struct FlexibleText(String);

  impl postgres_types::ToSql for FlexibleText {
    fn to_sql(
      &self,
      ty: &postgres_types::Type,
      out: &mut bytes::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Send + Sync>> {
      if *ty == postgres_types::Type::UUID {
        let uuid: uuid::Uuid = self.0.parse()?;
        uuid.to_sql(ty, out)
      } else {
        self.0.to_sql(ty, out)
      }
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
      *ty == postgres_types::Type::UUID || <String as postgres_types::ToSql>::accepts(ty)
    }

    postgres_types::to_sql_checked!();
  }

  /// Convert a slice of [`Value`] into boxed [`ToSql`] trait objects for
  /// `tokio_postgres::Client::query` / `execute`.
  pub fn to_pg_params(params: &[Value]) -> Vec<Box<dyn ToSql + Sync + Send>> {
    params
      .iter()
      .map(|v| -> Box<dyn ToSql + Sync + Send> {
        match v {
          Value::Null => Box::new(PgNull),
          Value::Integer(n) => Box::new(*n),
          Value::Real(f) => Box::new(*f),
          Value::Text(s) => Box::new(FlexibleText(s.clone())),
          Value::Blob(b) => Box::new(b.clone()),
        }
      })
      .collect()
  }
}

#[cfg(feature = "postgres")]
pub use pg_conversions::to_pg_params;
