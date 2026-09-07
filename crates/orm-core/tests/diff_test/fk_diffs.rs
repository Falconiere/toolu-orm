use std::collections::BTreeMap;

use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::diff::{diff_foreign_keys, Operation};
use toolu_orm_core::snapshot::{ForeignKeyDef, Snapshot, SnapshotMeta, SnapshotTable};

use super::col;

fn snapshot_with_fks(
  table_name: &str,
  columns: &[ColumnDef],
  fks: BTreeMap<String, ForeignKeyDef>,
) -> Snapshot {
  let mut cols_map = BTreeMap::new();
  for c in columns {
    cols_map.insert(c.name.clone(), c.clone());
  }
  let column_order: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
  let mut tables = BTreeMap::new();
  tables.insert(
    table_name.to_owned(),
    SnapshotTable {
      column_order,
      columns: cols_map,
      indexes: BTreeMap::new(),
      foreign_keys: fks,
      check_constraints: BTreeMap::new(),
      strict: false,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  );
  Snapshot {
    version: 1,
    dialect: "postgres".to_owned(),
    id: "test".to_owned(),
    prev_id: "prev".to_owned(),
    tables,
    enums: BTreeMap::new(),
    meta: SnapshotMeta::default(),
  }
}

#[test]
fn diff_add_foreign_key() {
  let columns = vec![
    col("id", ColumnType::Text, true, true),
    col("author_id", ColumnType::Text, false, true),
  ];

  let old = snapshot_with_fks("posts", &columns, BTreeMap::new());

  let mut new_fks = BTreeMap::new();
  new_fks.insert(
    "fk_posts_author".to_owned(),
    ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::Cascade),
      on_update: None,
    },
  );

  let new = snapshot_with_fks("posts", &columns, new_fks);
  let ops = diff_foreign_keys("posts", &old, &new);

  match ops.as_slice() {
    [Operation::AddForeignKey { table, fk }] => {
      assert_eq!(table, "posts");
      assert_eq!(fk.name, "fk_posts_author");
    },
    _ => assert_eq!(ops.len(), 1, "expected exactly one AddForeignKey operation"),
  }
}

#[test]
fn diff_drop_foreign_key() {
  let columns = vec![
    col("id", ColumnType::Text, true, true),
    col("author_id", ColumnType::Text, false, true),
  ];

  let mut old_fks = BTreeMap::new();
  old_fks.insert(
    "fk_posts_author".to_owned(),
    ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::Cascade),
      on_update: None,
    },
  );

  let old = snapshot_with_fks("posts", &columns, old_fks);
  let new = snapshot_with_fks("posts", &columns, BTreeMap::new());
  let ops = diff_foreign_keys("posts", &old, &new);

  match ops.as_slice() {
    [Operation::DropForeignKey { table, name }] => {
      assert_eq!(table, "posts");
      assert_eq!(name, "fk_posts_author");
    },
    _ => assert_eq!(
      ops.len(),
      1,
      "expected exactly one DropForeignKey operation"
    ),
  }
}

#[test]
fn diff_foreign_key_modified_emits_drop_and_add() {
  let columns = vec![
    col("id", ColumnType::Text, true, true),
    col("author_id", ColumnType::Text, false, true),
  ];

  let mut old_fks = BTreeMap::new();
  old_fks.insert(
    "fk_posts_author".to_owned(),
    ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::SetNull),
      on_update: None,
    },
  );

  let mut new_fks = BTreeMap::new();
  new_fks.insert(
    "fk_posts_author".to_owned(),
    ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::Cascade),
      on_update: None,
    },
  );

  let old = snapshot_with_fks("posts", &columns, old_fks);
  let new = snapshot_with_fks("posts", &columns, new_fks);
  let ops = diff_foreign_keys("posts", &old, &new);

  assert_eq!(ops.len(), 2);
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::DropForeignKey { .. })));
  assert!(ops
    .iter()
    .any(|op| matches!(op, Operation::AddForeignKey { .. })));
}

#[test]
fn diff_foreign_key_unchanged() {
  let columns = vec![
    col("id", ColumnType::Text, true, true),
    col("author_id", ColumnType::Text, false, true),
  ];

  let mut fks = BTreeMap::new();
  fks.insert(
    "fk_posts_author".to_owned(),
    ForeignKeyDef {
      name: "fk_posts_author".to_owned(),
      columns: vec!["author_id".to_owned()],
      references_table: "users".to_owned(),
      references_columns: vec!["id".to_owned()],
      on_delete: Some(ForeignKeyAction::Cascade),
      on_update: None,
    },
  );

  let old = snapshot_with_fks("posts", &columns, fks.clone());
  let new = snapshot_with_fks("posts", &columns, fks);
  let ops = diff_foreign_keys("posts", &old, &new);

  assert!(ops.is_empty());
}
