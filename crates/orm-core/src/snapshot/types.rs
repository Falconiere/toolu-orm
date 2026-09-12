//! Snapshot structs and conversions.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::column::{ColumnDef, ForeignKeyAction};
use crate::error::DbCoreError;
use crate::index::IndexDef;
use crate::schema::SchemaRegistry;
use crate::table::{TableDef, TableKind};

use super::extract;
use super::serde_compat;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignKeyDef {
  pub name: String,
  pub columns: Vec<String>,
  pub references_table: String,
  pub references_columns: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub on_delete: Option<ForeignKeyAction>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub on_update: Option<ForeignKeyAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEnum {
  pub name: String,
  pub variants: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMeta {
  #[serde(default)]
  pub tables: BTreeMap<String, String>,
  #[serde(default)]
  pub columns: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
  pub version: u32,
  #[serde(default = "default_sqlite_dialect")]
  pub dialect: String,
  #[serde(default)]
  pub id: String,
  #[serde(default)]
  pub prev_id: String,
  pub tables: BTreeMap<String, SnapshotTable>,
  #[serde(default)]
  pub enums: BTreeMap<String, SnapshotEnum>,
  #[serde(default)]
  pub meta: SnapshotMeta,
}

fn default_sqlite_dialect() -> String {
  "sqlite".to_owned()
}

fn ordered_columns(snap: &SnapshotTable) -> Vec<ColumnDef> {
  let mut out = Vec::new();
  let mut seen = BTreeSet::new();
  for name in &snap.column_order {
    let Some(col) = snap.columns.get(name) else {
      continue;
    };
    out.push(col.clone());
    seen.insert(name.clone());
  }
  for name in snap.columns.keys() {
    if seen.contains(name) {
      continue;
    }
    let Some(col) = snap.columns.get(name) else {
      continue;
    };
    out.push(col.clone());
    seen.insert(name.clone());
  }
  out
}

#[derive(Debug, Clone)]
pub struct SnapshotTable {
  /// Declared column order for `to_registry` (matches source `TableDef.columns`).
  pub column_order: Vec<String>,
  pub columns: BTreeMap<String, ColumnDef>,
  pub indexes: BTreeMap<String, IndexDef>,
  pub foreign_keys: BTreeMap<String, ForeignKeyDef>,
  pub check_constraints: BTreeMap<String, String>,
  /// Table-level composite primary key; empty when the key is per-column.
  pub primary_key: Vec<String>,
  pub strict: bool,
  /// [`TableKind::Ordinary`] for every table written before virtual tables
  /// existed, and omitted from the JSON when ordinary.
  pub kind: TableKind,
}

impl serde::Serialize for SnapshotTable {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serde_compat::serialize_snapshot_table(self, serializer)
  }
}

impl<'de> serde::Deserialize<'de> for SnapshotTable {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    serde_compat::deserialize_snapshot_table(deserializer)
  }
}

impl Snapshot {
  #[must_use]
  pub fn empty() -> Self {
    Self {
      version: 1,
      dialect: "sqlite".to_owned(),
      id: uuid::Uuid::new_v4().to_string(),
      prev_id: "00000000-0000-0000-0000-000000000000".to_owned(),
      tables: BTreeMap::new(),
      enums: BTreeMap::new(),
      meta: SnapshotMeta::default(),
    }
  }

  pub fn from_registry(registry: &SchemaRegistry) -> Self {
    let mut tables = BTreeMap::new();
    for table in registry.tables() {
      let column_order: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
      let fks = extract::extract_foreign_keys(&table.name, &table.columns);
      let checks = extract::extract_check_constraints(&table.columns);
      let columns = extract::columns_for_snapshot_table(&table.columns);
      let indexes: BTreeMap<String, IndexDef> = table
        .indexes
        .iter()
        .map(|i| (i.name.clone(), i.clone()))
        .collect();
      tables.insert(
        table.name.clone(),
        SnapshotTable {
          column_order,
          columns,
          indexes,
          foreign_keys: fks,
          check_constraints: checks,
          primary_key: table.primary_key.clone(),
          strict: table.strict,
          kind: table.kind.clone(),
        },
      );
    }
    Self {
      version: 1,
      dialect: "sqlite".to_owned(),
      id: uuid::Uuid::new_v4().to_string(),
      prev_id: "00000000-0000-0000-0000-000000000000".to_owned(),
      tables,
      enums: BTreeMap::new(),
      meta: SnapshotMeta::default(),
    }
  }

  pub fn to_registry(&self) -> SchemaRegistry {
    let tables: Vec<TableDef> = self
      .tables
      .iter()
      .map(|(name, snap_table)| {
        let mut snap_table = snap_table.clone();
        extract::merge_fk_into_columns(&mut snap_table);
        extract::merge_checks_into_columns(&mut snap_table);
        TableDef {
          name: name.clone(),
          columns: ordered_columns(&snap_table),
          indexes: snap_table.indexes.values().cloned().collect(),
          primary_key: snap_table.primary_key.clone(),
          strict: snap_table.strict,
          kind: snap_table.kind.clone(),
        }
      })
      .collect();
    SchemaRegistry::from_tables(tables)
  }

  /// Reads a snapshot from the given path. Returns an empty snapshot if the file does not exist.
  ///
  /// # Errors
  ///
  /// Returns `DbCoreError::SnapshotRead` if the file exists but cannot be read or parsed.
  pub fn read_from_path(path: &str) -> Result<Self, DbCoreError> {
    let p = Path::new(path);
    if !p.exists() {
      return Ok(Self::empty());
    }
    let content =
      std::fs::read_to_string(p).map_err(|e| DbCoreError::SnapshotRead(format!("{path}: {e}")))?;
    serde_json::from_str(&content).map_err(|e| DbCoreError::SnapshotRead(format!("{path}: {e}")))
  }

  /// Writes the snapshot to the given path as pretty-printed JSON.
  ///
  /// # Errors
  ///
  /// Returns `DbCoreError::SnapshotWrite` if the file cannot be written.
  pub fn write_to_path(&self, path: &str) -> Result<(), DbCoreError> {
    let json = serde_json::to_string_pretty(self)
      .map_err(|e| DbCoreError::SnapshotWrite(format!("{path}: {e}")))?;
    std::fs::write(path, json).map_err(|e| DbCoreError::SnapshotWrite(format!("{path}: {e}")))
  }
}
