//! Strict FLOAT[N] materialization for checked portable vectors.
use crate::error::DbError;
use duckdb::{
  arrow::array::{Array, Float32Array},
  types::ValueRef,
};
use toolu_orm_core::{value::Value, vector::VectorValue};

pub(super) fn decode(name: &str, native: ValueRef<'_>) -> Result<Value, DbError> {
  let error = |reason: &str| {
    DbError::RowMapping(format!(
      "Lance column {name}: {reason}; expected finite FLOAT[N]"
    ))
  };
  let ValueRef::Array(array, row) = native else {
    return Err(error("not a fixed array"));
  };
  let cell = array.value(row);
  let floats = cell
    .as_any()
    .downcast_ref::<Float32Array>()
    .ok_or_else(|| error("not a FLOAT array"))?;
  if floats.null_count() > 0 {
    return Err(error("NULL element"));
  }
  let dim = u32::try_from(floats.len())
    .map_err(|cause| error(&format!("dimension exceeds u32: {cause}")))?;
  let vector = VectorValue::new(floats.values(), dim).map_err(|cause| error(&cause.to_string()))?;
  Ok(Value::Vector(vector))
}
