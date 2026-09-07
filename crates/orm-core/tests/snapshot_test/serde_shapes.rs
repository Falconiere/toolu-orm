use std::collections::BTreeMap;

use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::snapshot::{
  ForeignKeyDef, Snapshot, SnapshotEnum, SnapshotMeta, SnapshotTable,
};

#[test]
fn snapshot_roundtrip_with_btreemap_columns() -> Result<(), Box<dyn std::error::Error>> {
  let mut columns = BTreeMap::new();
  columns.insert(
    "id".to_owned(),
    ColumnDef {
      name: "id".to_owned(),
      column_type: ColumnType::Text,
      primary_key: true,
      not_null: true,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
    },
  );
  columns.insert(
    "name".to_owned(),
    ColumnDef {
      name: "name".to_owned(),
      column_type: ColumnType::Text,
      primary_key: false,
      not_null: false,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: None,
    },
  );

  let table = SnapshotTable {
    column_order: vec!["id".to_owned(), "name".to_owned()],
    columns,
    indexes: BTreeMap::new(),
    foreign_keys: BTreeMap::new(),
    check_constraints: BTreeMap::new(),
    strict: false,
  };

  let mut tables = BTreeMap::new();
  tables.insert("users".to_owned(), table);

  let snapshot = Snapshot {
    version: 1,
    dialect: "sqlite".to_owned(),
    id: "abc-123".to_owned(),
    prev_id: "000-000".to_owned(),
    tables,
    enums: BTreeMap::new(),
    meta: SnapshotMeta {
      tables: BTreeMap::new(),
      columns: BTreeMap::new(),
    },
  };

  let json = serde_json::to_string_pretty(&snapshot)?;
  let parsed: Snapshot = serde_json::from_str(&json)?;
  assert_eq!(parsed.version, 1);
  assert_eq!(parsed.dialect, "sqlite");
  assert_eq!(parsed.id, "abc-123");
  assert_eq!(parsed.prev_id, "000-000");
  assert_eq!(parsed.tables.len(), 1);
  let users = parsed.tables.get("users").ok_or("missing users table")?;
  assert_eq!(users.columns.len(), 2);
  assert!(users.columns.contains_key("id"));
  assert!(users.columns.contains_key("name"));
  Ok(())
}

#[test]
fn snapshot_empty_has_default_fields() {
  let snap = Snapshot::empty();
  assert_eq!(snap.version, 1);
  assert_eq!(snap.dialect, "sqlite");
  assert!(snap.tables.is_empty());
  assert!(snap.enums.is_empty());
  assert!(snap.meta.tables.is_empty());
  assert!(snap.meta.columns.is_empty());
  assert!(!snap.id.is_empty());
  assert_eq!(snap.prev_id, "00000000-0000-0000-0000-000000000000");
}

#[test]
fn snapshot_indexes_use_btreemap() -> Result<(), Box<dyn std::error::Error>> {
  let mut indexes = BTreeMap::new();
  indexes.insert(
    "idx_email".to_owned(),
    IndexDef {
      name: "idx_email".to_owned(),
      columns: vec!["email".to_owned()],
      unique: true,
    },
  );

  let table = SnapshotTable {
    column_order: vec![],
    columns: BTreeMap::new(),
    indexes,
    foreign_keys: BTreeMap::new(),
    check_constraints: BTreeMap::new(),
    strict: false,
  };

  let json = serde_json::to_string(&table)?;
  let parsed: SnapshotTable = serde_json::from_str(&json)?;
  assert_eq!(parsed.indexes.len(), 1);
  assert!(parsed.indexes.contains_key("idx_email"));
  Ok(())
}

#[test]
fn foreign_key_def_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
  let fk = ForeignKeyDef {
    name: "fk_posts_author".to_owned(),
    columns: vec!["author_id".to_owned()],
    references_table: "users".to_owned(),
    references_columns: vec!["id".to_owned()],
    on_delete: Some(ForeignKeyAction::Cascade),
    on_update: None,
  };

  let json = serde_json::to_string(&fk)?;
  let parsed: ForeignKeyDef = serde_json::from_str(&json)?;
  assert_eq!(parsed.name, "fk_posts_author");
  assert_eq!(parsed.columns, vec!["author_id"]);
  assert_eq!(parsed.references_table, "users");
  assert_eq!(parsed.references_columns, vec!["id"]);
  assert_eq!(parsed.on_delete, Some(ForeignKeyAction::Cascade));
  assert_eq!(parsed.on_update, None);
  Ok(())
}

#[test]
fn snapshot_enum_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
  let e = SnapshotEnum {
    name: "status".to_owned(),
    variants: vec![
      "draft".to_owned(),
      "active".to_owned(),
      "archived".to_owned(),
    ],
  };

  let json = serde_json::to_string(&e)?;
  let parsed: SnapshotEnum = serde_json::from_str(&json)?;
  assert_eq!(parsed.name, "status");
  assert_eq!(parsed.variants, vec!["draft", "active", "archived"]);
  Ok(())
}

#[test]
fn snapshot_meta_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
  let mut meta = SnapshotMeta::default();
  meta
    .tables
    .insert("old_users".to_owned(), "users".to_owned());
  meta
    .columns
    .insert("users.old_name".to_owned(), "users.full_name".to_owned());

  let json = serde_json::to_string(&meta)?;
  let parsed: SnapshotMeta = serde_json::from_str(&json)?;
  assert_eq!(parsed.tables.get("old_users"), Some(&"users".to_owned()));
  assert_eq!(
    parsed.columns.get("users.old_name"),
    Some(&"users.full_name".to_owned())
  );
  Ok(())
}

#[test]
fn snapshot_meta_default_is_empty() {
  let meta = SnapshotMeta::default();
  assert!(meta.tables.is_empty());
  assert!(meta.columns.is_empty());
}

#[test]
fn snapshot_with_enums() -> Result<(), Box<dyn std::error::Error>> {
  let mut enums = BTreeMap::new();
  enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned(), "published".to_owned()],
    },
  );

  let snapshot = Snapshot {
    version: 1,
    dialect: "postgres".to_owned(),
    id: "test-id".to_owned(),
    prev_id: "prev-id".to_owned(),
    tables: BTreeMap::new(),
    enums,
    meta: SnapshotMeta::default(),
  };

  let json = serde_json::to_string_pretty(&snapshot)?;
  let parsed: Snapshot = serde_json::from_str(&json)?;
  assert_eq!(parsed.enums.len(), 1);
  let status = parsed.enums.get("status").ok_or("missing status enum")?;
  assert_eq!(status.variants, vec!["draft", "published"]);
  Ok(())
}
