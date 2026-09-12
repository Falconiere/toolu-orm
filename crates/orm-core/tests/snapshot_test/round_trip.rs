use toolu_orm_core::column::ColumnDef;
use toolu_orm_core::column::ColumnType;
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableDef;

use super::registry_samples::{sample_registry, TestResult};

#[test]
fn test_snapshot_round_trip() -> TestResult {
  let registry = sample_registry();
  let snapshot = Snapshot::from_registry(&registry);
  let json = serde_json::to_string_pretty(&snapshot)?;
  let restored: Snapshot = serde_json::from_str(&json)?;
  let restored_registry = restored.to_registry();
  assert_eq!(registry.tables().len(), restored_registry.tables().len());
  let original_table = registry
    .find_table("conversations")
    .ok_or("missing conversations table in original")?;
  let restored_table = restored_registry
    .find_table("conversations")
    .ok_or("missing conversations table in restored")?;
  assert_eq!(original_table, restored_table);
  Ok(())
}

#[test]
fn test_snapshot_from_empty_registry() {
  let registry = SchemaRegistry::from_tables(vec![]);
  let snapshot = Snapshot::from_registry(&registry);
  assert_eq!(snapshot.version, 1);
  assert!(snapshot.tables.is_empty());
}

#[test]
fn test_snapshot_read_missing_file_returns_empty() -> TestResult {
  let snapshot = Snapshot::read_from_path("/tmp/nonexistent_snapshot_test_abc123.json")?;
  assert!(snapshot.tables.is_empty());
  Ok(())
}

#[test]
fn test_snapshot_write_and_read() -> TestResult {
  let registry = sample_registry();
  let snapshot = Snapshot::from_registry(&registry);
  let path = "/tmp/db_core_snapshot_test.json";
  snapshot.write_to_path(path)?;
  let loaded = Snapshot::read_from_path(path)?;
  assert_eq!(loaded.tables.len(), 1);
  assert!(loaded.tables.contains_key("conversations"));
  std::fs::remove_file(path).ok();
  Ok(())
}

#[test]
fn test_snapshot_round_trip_with_indexes() -> Result<(), Box<dyn std::error::Error>> {
  let reg = SchemaRegistry::from_tables(vec![TableDef {
    name: "pipelines".to_owned(),
    columns: vec![ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Uuid,
      primary_key: true,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![IndexDef {
      name: "idx_repo".to_owned(),
      columns: vec!["repo_id".into()],
      unique: false,
      where_clause: None,
    }],
    primary_key: vec![],
    strict: true,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }]);
  let snap = Snapshot::from_registry(&reg);
  let restored = snap.to_registry();
  let t = restored
    .find_table("pipelines")
    .ok_or("missing pipelines table")?;
  assert_eq!(t.indexes.len(), 1);
  assert!(t.strict);
  Ok(())
}
