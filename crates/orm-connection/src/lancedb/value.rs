//! Fallible conversion of portable ORM values into owned DuckDB parameters.

use duckdb::types::Value as DuckValue;
use toolu_orm_core::value::Value;

/// A portable value has no safe Lance scalar binding in this epic.
#[derive(Debug, thiserror::Error)]
pub enum LanceValueError {
  /// The tagged codec is deferred rather than silently bound as another type.
  #[error("LanceUnsupportedValue: {kind} has no DuckDB–Lance scalar binding")]
  Unsupported { kind: &'static str },
}

/// Convert a complete slice before handing it to a prepared DuckDB statement.
///
/// The returned values bind with `duckdb::params_from_iter(values.iter())`.
/// Conversion does not execute SQL, so an unsupported value cannot cause a
/// partial write. The caller must convert before executing the statement.
///
/// # Errors
///
/// [`LanceValueError::Unsupported`] for timestamp, JSON, UUID, or numeric
/// variants. These tagged codecs are outside epic #145.
pub fn to_duckdb_params(params: &[Value]) -> Result<Vec<DuckValue>, LanceValueError> {
  params.iter().map(convert_one).collect()
}

fn convert_one(value: &Value) -> Result<DuckValue, LanceValueError> {
  match value {
    Value::Null => Ok(DuckValue::Null),
    Value::Integer(number) => Ok(DuckValue::BigInt(*number)),
    Value::Real(number) => Ok(DuckValue::Double(*number)),
    Value::Text(text) => Ok(DuckValue::Text(text.clone())),
    Value::Blob(bytes) => Ok(DuckValue::Blob(bytes.clone())),
    Value::Boolean(boolean) => Ok(DuckValue::Boolean(*boolean)),
    Value::TimestampEpoch(_) => Err(unsupported("TimestampEpoch")),
    Value::TimestampText(_) => Err(unsupported("TimestampText")),
    Value::Json { .. } => Err(unsupported("Json")),
    Value::Uuid(_) => Err(unsupported("Uuid")),
    Value::Numeric(_) => Err(unsupported("Numeric")),
  }
}

fn unsupported(kind: &'static str) -> LanceValueError {
  LanceValueError::Unsupported { kind }
}
