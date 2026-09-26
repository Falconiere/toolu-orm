//! Convert supported Lance result scalars to portable owned row values.

use duckdb::{Row, types::ValueRef};
use toolu_orm_core::{row::LanceRow, value::Value};

use crate::error::DbError;

/// Materialize a projected DuckDB row for a portable `FromRow` decoder.
pub(super) fn portable_row(names: &[String], row: &Row<'_>) -> Result<LanceRow, DbError> {
  let mut columns = Vec::with_capacity(names.len());
  for (index, name) in names.iter().enumerate() {
    let native = row
      .get_ref(index)
      .map_err(|error| DbError::RowMapping(format!("Lance column {name}: {error}")))?;
    columns.push((name.clone(), scalar(name, native)?));
  }
  LanceRow::from_columns(columns).map_err(DbError::from)
}

fn scalar(name: &str, native: ValueRef<'_>) -> Result<Value, DbError> {
  let overflow = |_| {
    DbError::RowMapping(format!(
      "Lance column {name}: integer outside portable i64 range"
    ))
  };
  Ok(match native {
    ValueRef::Null => Value::Null,
    ValueRef::Boolean(value) => Value::Boolean(value),
    ValueRef::TinyInt(value) => Value::Integer(i64::from(value)),
    ValueRef::SmallInt(value) => Value::Integer(i64::from(value)),
    ValueRef::Int(value) => Value::Integer(i64::from(value)),
    ValueRef::BigInt(value) => Value::Integer(value),
    ValueRef::UTinyInt(value) => Value::Integer(i64::from(value)),
    ValueRef::USmallInt(value) => Value::Integer(i64::from(value)),
    ValueRef::UInt(value) => Value::Integer(i64::from(value)),
    ValueRef::UBigInt(value) => Value::Integer(i64::try_from(value).map_err(overflow)?),
    ValueRef::HugeInt(value) => Value::Integer(i64::try_from(value).map_err(overflow)?),
    ValueRef::UHugeInt(value) => Value::Integer(i64::try_from(value).map_err(overflow)?),
    ValueRef::Float(value) => Value::Real(f64::from(value)),
    ValueRef::Double(value) => Value::Real(value),
    ValueRef::Text(bytes) => Value::Text(
      std::str::from_utf8(bytes)
        .map_err(|error| DbError::RowMapping(format!("Lance column {name}: {error}")))?
        .to_owned(),
    ),
    ValueRef::Blob(bytes) => Value::Blob(bytes.to_vec()),
    ValueRef::Decimal(_)
    | ValueRef::Timestamp(..)
    | ValueRef::Geometry(_)
    | ValueRef::Date32(_)
    | ValueRef::Time64(..)
    | ValueRef::Interval { .. }
    | ValueRef::List(..)
    | ValueRef::Enum(..)
    | ValueRef::Struct(..)
    | ValueRef::Array(..)
    | ValueRef::Map(..)
    | ValueRef::Union(..)
    | _ => {
      return Err(DbError::RowMapping(format!(
        "Lance column {name}: unsupported result type {:?}",
        native.data_type()
      )));
    },
  })
}
