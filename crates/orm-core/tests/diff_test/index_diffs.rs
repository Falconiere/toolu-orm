//! Tests for index-level diff operations: create, drop, alter index.
//!
//! # Public API
//!
//! Tests for diff() producing CreateIndex, DropIndex operations and
//! combined create-table-with-indexes.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;

use super::{col, table};

#[test]
fn test_diff_create_index() {
  let old_reg = SchemaRegistry::from_tables(vec![table(
    "t",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let old = Snapshot::from_registry(&old_reg);
  let mut new_t = table("t", vec![col("id", ColumnType::Text, true, false)]);
  new_t.indexes = vec![IndexDef {
    name: "idx_t_id".to_owned(),
    columns: vec!["id".to_owned()],
    unique: false,
    where_clause: None,
  }];
  let new_reg = SchemaRegistry::from_tables(vec![new_t]);
  let ops = diff(&old, &new_reg).expect("diff should succeed");
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::CreateIndex { .. })));
}

#[test]
fn test_diff_drop_index() {
  let mut old_t = table("t", vec![col("id", ColumnType::Text, true, false)]);
  old_t.indexes = vec![IndexDef {
    name: "idx_old".to_owned(),
    columns: vec!["id".to_owned()],
    unique: false,
    where_clause: None,
  }];
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![old_t]));
  let new_reg = SchemaRegistry::from_tables(vec![table(
    "t",
    vec![col("id", ColumnType::Text, true, false)],
  )]);
  let ops = diff(&old, &new_reg).expect("diff should succeed");
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::DropIndex { .. })));
}

#[test]
fn test_diff_alter_index() {
  let mut old_t = table("t", vec![col("id", ColumnType::Text, true, false)]);
  old_t.indexes = vec![IndexDef {
    name: "idx_t_id".to_owned(),
    columns: vec!["id".to_owned()],
    unique: false,
    where_clause: None,
  }];
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![old_t]));
  let mut new_t = table("t", vec![col("id", ColumnType::Text, true, false)]);
  new_t.indexes = vec![IndexDef {
    name: "idx_t_id".to_owned(),
    columns: vec!["id".to_owned()],
    unique: true,
    where_clause: None,
  }];
  let new_reg = SchemaRegistry::from_tables(vec![new_t]);
  let ops = diff(&old, &new_reg).expect("diff should succeed");
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::DropIndex { name } if name == "idx_t_id")));
  assert!(ops.iter().any(
    |op| matches!(op, Operation::CreateIndex { table, index } if table == "t" && index.unique)
  ));
}

#[test]
fn test_diff_create_table_emits_indexes() {
  let old = Snapshot::from_registry(&SchemaRegistry::from_tables(vec![]));
  let mut new_t = table("t", vec![col("id", ColumnType::Text, true, false)]);
  new_t.indexes = vec![IndexDef {
    name: "idx_t_id".to_owned(),
    columns: vec!["id".to_owned()],
    unique: false,
    where_clause: None,
  }];
  let new_reg = SchemaRegistry::from_tables(vec![new_t]);
  let ops = diff(&old, &new_reg).expect("diff should succeed");
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::CreateTable { .. })));
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::CreateIndex { .. })));
}
