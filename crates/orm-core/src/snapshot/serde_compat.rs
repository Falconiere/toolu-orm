//! Backward-compatible JSON (de)serialization for [`super::types::SnapshotTable`].

use std::collections::BTreeMap;

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::column::ColumnDef;
use crate::index::IndexDef;
use crate::table::TableKind;

use super::types::{ForeignKeyDef, SnapshotTable};

pub(crate) fn serialize_snapshot_table<S>(
  table: &SnapshotTable,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  #[derive(Serialize)]
  struct SnapshotTableSer<'a> {
    column_order: &'a [String],
    columns: &'a BTreeMap<String, ColumnDef>,
    indexes: &'a BTreeMap<String, IndexDef>,
    foreign_keys: &'a BTreeMap<String, ForeignKeyDef>,
    check_constraints: &'a BTreeMap<String, String>,
    strict: bool,
    #[serde(skip_serializing_if = "TableKind::is_ordinary")]
    kind: &'a TableKind,
  }
  SnapshotTableSer {
    column_order: &table.column_order,
    columns: &table.columns,
    indexes: &table.indexes,
    foreign_keys: &table.foreign_keys,
    check_constraints: &table.check_constraints,
    strict: table.strict,
    kind: &table.kind,
  }
  .serialize(serializer)
}

pub(crate) fn deserialize_snapshot_table<'de, D>(deserializer: D) -> Result<SnapshotTable, D::Error>
where
  D: Deserializer<'de>,
{
  #[derive(Deserialize)]
  struct Wire {
    #[serde(default)]
    column_order: Vec<String>,
    columns: serde_json::Value,
    #[serde(default)]
    indexes: serde_json::Value,
    #[serde(default)]
    foreign_keys: BTreeMap<String, ForeignKeyDef>,
    #[serde(default)]
    check_constraints: BTreeMap<String, String>,
    #[serde(default)]
    strict: bool,
    #[serde(default)]
    kind: TableKind,
  }
  let w = Wire::deserialize(deserializer)?;
  let (columns, inferred_order) =
    parse_columns_value(w.columns).map_err(serde::de::Error::custom)?;
  let column_order = if w.column_order.is_empty() {
    if inferred_order.is_empty() {
      columns.keys().cloned().collect()
    } else {
      inferred_order
    }
  } else {
    w.column_order
  };
  let indexes = parse_indexes_value(w.indexes).map_err(serde::de::Error::custom)?;
  Ok(SnapshotTable {
    column_order,
    columns,
    indexes,
    foreign_keys: w.foreign_keys,
    check_constraints: w.check_constraints,
    strict: w.strict,
    kind: w.kind,
  })
}

fn parse_columns_value(
  v: serde_json::Value,
) -> Result<(BTreeMap<String, ColumnDef>, Vec<String>), String> {
  if v.is_null() {
    return Ok((BTreeMap::new(), vec![]));
  }
  if v.is_array() {
    let vec: Vec<ColumnDef> =
      serde_json::from_value(v).map_err(|e| format!("columns array: {e}"))?;
    let order: Vec<String> = vec.iter().map(|c| c.name.clone()).collect();
    let map: BTreeMap<String, ColumnDef> = vec.into_iter().map(|c| (c.name.clone(), c)).collect();
    return Ok((map, order));
  }
  let map: BTreeMap<String, ColumnDef> =
    serde_json::from_value(v).map_err(|e| format!("columns object: {e}"))?;
  Ok((map, vec![]))
}

fn parse_indexes_value(v: serde_json::Value) -> Result<BTreeMap<String, IndexDef>, String> {
  if v.is_null() {
    return Ok(BTreeMap::new());
  }
  if v.is_array() {
    let vec: Vec<IndexDef> = serde_json::from_value(v).map_err(|e| format!("indexes: {e}"))?;
    return Ok(vec.into_iter().map(|i| (i.name.clone(), i)).collect());
  }
  serde_json::from_value(v).map_err(|e| format!("indexes: {e}"))
}
