//! Decoding of relation columns whose projection declares binary columns.

use serde_json::Value;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::relational_row::{parse_json_array_of_arrays, parse_json_single_array};

use super::config::{RelationConfig, RelationalSelectBuilder};

impl RelationalSelectBuilder {
  /// Decode one relation column from the JSON text SQLite returns for it.
  ///
  /// Yields what `from_relational_values` expects for that column: rows as
  /// arrays for a has-many, one row array or `null` for a has-one. Columns
  /// declared with [`RelationColumn::binary`](super::RelationColumn::binary)
  /// decode from hex into a JSON array of bytes; others pass through untouched.
  ///
  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] when `field_name` names no relation,
  /// when `json` is not the shape the relation's kind requires, or when a
  /// declared binary column holds something other than null or valid hex text.
  pub fn decode_relation_json(&self, field_name: &str, json: &str) -> Result<Value, DbCoreError> {
    let rel = self.require_relation(field_name)?;
    if rel.is_many {
      let rows = parse_json_array_of_arrays(json)?;
      decode_many(rel, rows)
    } else {
      let row = parse_json_single_array(json)?;
      decode_one(rel, row)
    }
  }

  /// Decode one relation column already parsed as JSON.
  ///
  /// Postgres returns relation columns as `json`; a SQL NULL is passed as
  /// [`Value::Null`]. Otherwise identical to [`Self::decode_relation_json`].
  ///
  /// # Errors
  ///
  /// Same as [`Self::decode_relation_json`].
  pub fn decode_relation_value(
    &self,
    field_name: &str,
    column: &Value,
  ) -> Result<Value, DbCoreError> {
    let rel = self.require_relation(field_name)?;
    if column.is_null() {
      return Ok(if rel.is_many {
        Value::Array(Vec::new())
      } else {
        Value::Null
      });
    }
    let rows = column
      .as_array()
      .ok_or_else(|| row_error(field_name, "expected a JSON array"))?;

    if rel.is_many {
      let mut parsed = Vec::with_capacity(rows.len());
      for row in rows {
        let inner = row
          .as_array()
          .ok_or_else(|| row_error(field_name, "expected an inner JSON array per row"))?;
        parsed.push(inner.clone());
      }
      decode_many(rel, parsed)
    } else {
      decode_one(rel, rows.clone())
    }
  }

  fn require_relation(&self, field_name: &str) -> Result<&RelationConfig, DbCoreError> {
    self
      .relation(field_name)
      .ok_or_else(|| DbCoreError::RowMapping(format!("unknown relation field '{field_name}'")))
  }
}

fn decode_many(rel: &RelationConfig, rows: Vec<Vec<Value>>) -> Result<Value, DbCoreError> {
  let mut out = Vec::with_capacity(rows.len());
  for row in rows {
    out.push(Value::Array(decode_row(rel, row)?));
  }
  Ok(Value::Array(out))
}

fn decode_one(rel: &RelationConfig, row: Vec<Value>) -> Result<Value, DbCoreError> {
  if row.is_empty() {
    return Ok(Value::Null);
  }
  Ok(Value::Array(decode_row(rel, row)?))
}

/// Rewrites the declared binary positions of one relation row.
fn decode_row(rel: &RelationConfig, row: Vec<Value>) -> Result<Vec<Value>, DbCoreError> {
  if row.len() != rel.target_columns.len() {
    return Err(row_error(
      &rel.field_name,
      &format!(
        "got {} values for {} columns",
        row.len(),
        rel.target_columns.len()
      ),
    ));
  }
  let mut out = Vec::with_capacity(row.len());
  for (col, value) in rel.target_columns.iter().zip(row) {
    if col.is_binary() {
      out.push(decode_binary(&rel.field_name, col.name(), &value)?);
    } else {
      out.push(value);
    }
  }
  Ok(out)
}

fn decode_binary(field_name: &str, column: &str, value: &Value) -> Result<Value, DbCoreError> {
  match value {
    Value::Null => Ok(Value::Null),
    Value::String(hex) => {
      let bytes = hex_to_bytes(hex).ok_or_else(|| {
        row_error(
          field_name,
          &format!("column '{column}' is not valid hex-encoded binary"),
        )
      })?;
      Ok(Value::Array(bytes.into_iter().map(Value::from).collect()))
    },
    Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => Err(row_error(
      field_name,
      &format!("column '{column}' is binary but the value is neither a string nor null"),
    )),
  }
}

/// Hex text (either case) to bytes; `None` for an odd length or a non-hex digit.
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
  let raw = hex.as_bytes();
  if raw.len() % 2 != 0 {
    return None;
  }
  let mut out = Vec::with_capacity(raw.len() / 2);
  for pair in raw.chunks_exact(2) {
    let [high, low] = pair else { return None };
    let high = u8::try_from(char::from(*high).to_digit(16)?).ok()?;
    let low = u8::try_from(char::from(*low).to_digit(16)?).ok()?;
    out.push(high * 16 + low);
  }
  Some(out)
}

fn row_error(field_name: &str, detail: &str) -> DbCoreError {
  DbCoreError::RowMapping(format!("relation '{field_name}': {detail}"))
}
