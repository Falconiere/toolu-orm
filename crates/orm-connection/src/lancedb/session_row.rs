//! Convert the small supported Lance result subset to portable row values.

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
    let value = if let ValueRef::Null = native {
      Value::Null
    } else if let ValueRef::BigInt(number) = native {
      Value::Integer(number)
    } else if let ValueRef::Text(bytes) = native {
      Value::Text(
        std::str::from_utf8(bytes)
          .map_err(|error| DbError::RowMapping(format!("Lance column {name}: {error}")))?
          .to_owned(),
      )
    } else {
      return Err(DbError::RowMapping(format!(
        "Lance column {name}: unsupported result type {:?}",
        native.data_type()
      )));
    };
    columns.push((name.clone(), value));
  }
  LanceRow::from_columns(columns).map_err(DbError::from)
}
