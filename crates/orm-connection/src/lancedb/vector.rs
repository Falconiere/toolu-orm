//! Strict FLOAT[N] materialization for checked portable vectors.
use crate::error::DbError;
use duckdb::types::{Type, Value as DuckValue, ValueRef};
use toolu_orm_core::{value::Value, vector::VectorValue};

pub(super) fn decode(name: &str, native: ValueRef<'_>) -> Result<Value, DbError> {
  let error = |reason: &str| {
    DbError::RowMapping(format!(
      "Lance column {name}: {reason}; expected finite FLOAT[N]"
    ))
  };
  if !matches!(native.data_type(), Type::Array(element, _) if *element == Type::Float) {
    return Err(error("not a FLOAT array"));
  }
  let DuckValue::Array(values) = native.to_owned() else {
    return Err(error("not a fixed array"));
  };
  let mut elements = Vec::with_capacity(values.len());
  for value in values {
    let DuckValue::Float(element) = value else {
      return Err(error("non-FLOAT or NULL element"));
    };
    elements.push(element);
  }
  let dim = u32::try_from(elements.len())
    .map_err(|cause| error(&format!("dimension exceeds u32: {cause}")))?;
  let vector = VectorValue::new(&elements, dim).map_err(|cause| error(&cause.to_string()))?;
  Ok(Value::Vector(vector))
}
