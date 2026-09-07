//! JSON-backed deserialization for relational SELECT results.
//!
//! # Public API
//!
//! - [`FromRelationalRow`], [`RelationDeserializer`]
//! - [`parse_json_array_of_arrays`], [`parse_json_single_array`], [`from_json_object_slice`]

use serde::de::DeserializeOwned;

use crate::error::DbCoreError;

/// Parse a JSON string containing an array of arrays (one inner array per row).
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when JSON is invalid or not an array of arrays.
pub fn parse_json_array_of_arrays(json: &str) -> Result<Vec<Vec<serde_json::Value>>, DbCoreError> {
  let parsed: serde_json::Value =
    serde_json::from_str(json).map_err(|e| DbCoreError::RowMapping(e.to_string()))?;

  let outer = parsed
    .as_array()
    .ok_or_else(|| DbCoreError::RowMapping("expected JSON array".to_owned()))?;

  let mut rows = Vec::with_capacity(outer.len());
  for item in outer {
    let inner = item
      .as_array()
      .ok_or_else(|| DbCoreError::RowMapping("expected inner array in JSON row".to_owned()))?
      .clone();
    rows.push(inner);
  }

  Ok(rows)
}

/// Parse a JSON string containing a single array of values (one row).
///
/// For `"null"` input returns an empty vec.
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when JSON is invalid or the top-level value is neither
/// `null` nor an array.
pub fn parse_json_single_array(json: &str) -> Result<Vec<serde_json::Value>, DbCoreError> {
  let parsed: serde_json::Value =
    serde_json::from_str(json).map_err(|e| DbCoreError::RowMapping(e.to_string()))?;

  if parsed.is_null() {
    return Ok(Vec::new());
  }

  parsed
    .as_array()
    .cloned()
    .ok_or_else(|| DbCoreError::RowMapping("expected JSON array or null".to_owned()))
}

/// Build a JSON object from positional `json_build_array` / `json_array` values and `T: Deserialize`.
///
/// # Errors
///
/// Returns [`DbCoreError::RowMapping`] when `values` and `keys` lengths differ or deserialization
/// fails.
pub fn from_json_object_slice<T: DeserializeOwned>(
  values: &[serde_json::Value],
  keys: &[&str],
) -> Result<T, DbCoreError> {
  if values.len() != keys.len() {
    return Err(DbCoreError::RowMapping(format!(
      "column count mismatch: got {} values for {} keys",
      values.len(),
      keys.len()
    )));
  }
  let mut map = serde_json::Map::new();
  for (k, v) in keys.iter().zip(values.iter()) {
    map.insert((*k).to_owned(), v.clone());
  }
  serde_json::from_value(serde_json::Value::Object(map))
    .map_err(|e| DbCoreError::RowMapping(e.to_string()))
}

/// Extract typed values from a parsed JSON array row.
pub struct RelationDeserializer<'a> {
  values: &'a [serde_json::Value],
}

impl<'a> RelationDeserializer<'a> {
  /// Create a deserializer over a slice of JSON values.
  pub fn new(values: &'a [serde_json::Value]) -> Self {
    Self { values }
  }

  /// Extract a typed value at the given index.
  ///
  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] when `index` is out of range or the value cannot be
  /// deserialized to `T`.
  pub fn get<T: DeserializeOwned>(&self, index: usize) -> Result<T, DbCoreError> {
    let val = self.values.get(index).ok_or_else(|| {
      DbCoreError::RowMapping(format!(
        "relation column index {index} out of bounds (len {})",
        self.values.len()
      ))
    })?;
    serde_json::from_value(val.clone())
      .map_err(|e| DbCoreError::RowMapping(format!("column {index}: {e}")))
  }

  /// Number of values in this row.
  pub fn len(&self) -> usize {
    self.values.len()
  }

  /// Whether this row has no values.
  pub fn is_empty(&self) -> bool {
    self.values.is_empty()
  }
}

/// Types deserialized from relational query rows (scalar + JSON relation columns).
pub trait FromRelationalRow: Sized {
  /// Column names for scalar (non-relation) fields, in order.
  const SCALAR_COLUMNS: &'static [&'static str];

  /// Construct `Self` from JSON values (scalars then relation columns).
  ///
  /// # Errors
  ///
  /// Returns [`DbCoreError::RowMapping`] when any scalar or relation column fails to deserialize.
  fn from_relational_values(values: &[serde_json::Value]) -> Result<Self, DbCoreError>;
}
