use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::diff::{ColumnChange, Operation};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::ordering::order_operations;
use toolu_orm_core::table::TableDef;

#[test]
fn create_enum_before_create_table() {
  let ops = vec![
    Operation::CreateTable {
      table: TableDef {
        name: "posts".to_owned(),
        columns: vec![],
        indexes: vec![],
        strict: false,
      },
    },
    Operation::CreateEnum {
      name: "status".to_owned(),
      variants: vec!["draft".to_owned()],
    },
  ];
  let ordered = order_operations(ops);
  match ordered.as_slice() {
    [a, b] => {
      assert!(matches!(a, Operation::CreateEnum { .. }));
      assert!(matches!(b, Operation::CreateTable { .. }));
    },
    _ => assert_eq!(ordered.len(), 2, "expected two ordered operations"),
  }
}

#[test]
fn drop_index_before_alter_column() {
  let ops = vec![
    Operation::AlterColumn {
      table: "users".to_owned(),
      changes: vec![ColumnChange::Type {
        column: "age".to_owned(),
        old: ColumnType::Integer,
        new: ColumnType::BigInt,
      }],
      table_def: TableDef {
        name: "users".to_owned(),
        columns: vec![],
        indexes: vec![],
        strict: false,
      },
    },
    Operation::DropIndex {
      name: "idx_users_age".to_owned(),
    },
  ];
  let ordered = order_operations(ops);
  match ordered.as_slice() {
    [a, b] => {
      assert!(matches!(a, Operation::DropIndex { .. }));
      assert!(matches!(b, Operation::AlterColumn { .. }));
    },
    _ => assert_eq!(ordered.len(), 2, "expected two ordered operations"),
  }
}

#[test]
fn drop_foreign_key_before_drop_column() {
  let ops = vec![
    Operation::DropColumn {
      table: "posts".to_owned(),
      column: "author_id".to_owned(),
    },
    Operation::DropForeignKey {
      table: "posts".to_owned(),
      name: "fk_posts_author".to_owned(),
    },
  ];
  let ordered = order_operations(ops);
  match ordered.as_slice() {
    [a, b] => {
      assert!(matches!(a, Operation::DropForeignKey { .. }));
      assert!(matches!(b, Operation::DropColumn { .. }));
    },
    _ => assert_eq!(ordered.len(), 2, "expected two ordered operations"),
  }
}

#[test]
fn add_column_before_create_index() {
  let ops = vec![
    Operation::CreateIndex {
      table: "users".to_owned(),
      index: IndexDef {
        name: "idx_email".to_owned(),
        columns: vec!["email".to_owned()],
        unique: true,
      },
    },
    Operation::AddColumn {
      table: "users".to_owned(),
      column: ColumnDef {
        name: "email".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: true,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
      },
    },
  ];
  let ordered = order_operations(ops);
  match ordered.as_slice() {
    [a, b] => {
      assert!(matches!(a, Operation::AddColumn { .. }));
      assert!(matches!(b, Operation::CreateIndex { .. }));
    },
    _ => assert_eq!(ordered.len(), 2, "expected two ordered operations"),
  }
}

#[test]
fn drop_table_before_drop_enum() {
  let ops = vec![
    Operation::DropEnum {
      name: "status".to_owned(),
    },
    Operation::DropTable {
      name: "posts".to_owned(),
    },
  ];
  let ordered = order_operations(ops);
  match ordered.as_slice() {
    [a, b] => {
      assert!(matches!(a, Operation::DropTable { .. }));
      assert!(matches!(b, Operation::DropEnum { .. }));
    },
    _ => assert_eq!(ordered.len(), 2, "expected two ordered operations"),
  }
}
