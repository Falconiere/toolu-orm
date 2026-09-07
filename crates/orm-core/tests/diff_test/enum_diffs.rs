use std::collections::BTreeMap;

use toolu_orm_core::diff::{diff_enums, Operation};
use toolu_orm_core::snapshot::{Snapshot, SnapshotEnum, SnapshotMeta};

fn empty_snapshot_with_enums(enums: BTreeMap<String, SnapshotEnum>) -> Snapshot {
  Snapshot {
    version: 1,
    dialect: "postgres".to_owned(),
    id: "test".to_owned(),
    prev_id: "prev".to_owned(),
    tables: BTreeMap::new(),
    enums,
    meta: SnapshotMeta::default(),
  }
}

#[test]
fn diff_create_enum() {
  let old = empty_snapshot_with_enums(BTreeMap::new());
  let mut new_enums = BTreeMap::new();
  new_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned(), "active".to_owned()],
    },
  );

  let new_snapshot = empty_snapshot_with_enums(new_enums);
  let ops = diff_enums(&old, &new_snapshot);

  match ops.as_slice() {
    [Operation::CreateEnum { name, variants }] => {
      assert_eq!(name, "status");
      assert_eq!(variants.len(), 2);
    },
    _ => assert_eq!(ops.len(), 1, "expected exactly one CreateEnum operation"),
  }
}

#[test]
fn diff_drop_enum() {
  let mut old_enums = BTreeMap::new();
  old_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned()],
    },
  );
  let old = empty_snapshot_with_enums(old_enums);
  let new = empty_snapshot_with_enums(BTreeMap::new());
  let ops = diff_enums(&old, &new);

  match ops.as_slice() {
    [Operation::DropEnum { name }] => assert_eq!(name, "status"),
    _ => assert_eq!(ops.len(), 1, "expected exactly one DropEnum operation"),
  }
}

#[test]
fn diff_alter_enum_add_variant() {
  let mut old_enums = BTreeMap::new();
  old_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned(), "active".to_owned()],
    },
  );

  let mut new_enums = BTreeMap::new();
  new_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec![
        "draft".to_owned(),
        "active".to_owned(),
        "archived".to_owned(),
      ],
    },
  );

  let old = empty_snapshot_with_enums(old_enums);
  let new = empty_snapshot_with_enums(new_enums);
  let ops = diff_enums(&old, &new);

  match ops.as_slice() {
    [Operation::AlterEnum {
      name,
      added,
      removed,
    }] => {
      assert_eq!(name, "status");
      assert_eq!(added, &vec!["archived".to_owned()]);
      assert!(removed.is_empty());
    },
    [wrong] => assert!(
      matches!(wrong, Operation::AlterEnum { .. }),
      "expected AlterEnum, got {wrong:?}"
    ),
    _ => assert_eq!(ops.len(), 1, "expected exactly one operation"),
  }
}

#[test]
fn diff_alter_enum_remove_variant() {
  let mut old_enums = BTreeMap::new();
  old_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec![
        "draft".to_owned(),
        "active".to_owned(),
        "old_status".to_owned(),
      ],
    },
  );

  let mut new_enums = BTreeMap::new();
  new_enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned(), "active".to_owned()],
    },
  );

  let old = empty_snapshot_with_enums(old_enums);
  let new = empty_snapshot_with_enums(new_enums);
  let ops = diff_enums(&old, &new);

  match ops.as_slice() {
    [Operation::AlterEnum {
      name,
      added,
      removed,
    }] => {
      assert_eq!(name, "status");
      assert!(added.is_empty());
      assert_eq!(removed, &vec!["old_status".to_owned()]);
    },
    [wrong] => assert!(
      matches!(wrong, Operation::AlterEnum { .. }),
      "expected AlterEnum, got {wrong:?}"
    ),
    _ => assert_eq!(ops.len(), 1, "expected exactly one operation"),
  }
}

#[test]
fn diff_enum_unchanged() {
  let mut enums = BTreeMap::new();
  enums.insert(
    "status".to_owned(),
    SnapshotEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned(), "active".to_owned()],
    },
  );

  let old = empty_snapshot_with_enums(enums.clone());
  let new = empty_snapshot_with_enums(enums);
  let ops = diff_enums(&old, &new);

  assert!(ops.is_empty());
}
