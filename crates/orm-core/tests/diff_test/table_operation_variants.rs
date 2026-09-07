use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::{ColumnChange, Operation};

use super::table;

#[test]
fn column_change_type_variant() {
  let change = ColumnChange::Type {
    column: "age".to_owned(),
    old: ColumnType::Integer,
    new: ColumnType::BigInt,
  };
  assert!(matches!(change, ColumnChange::Type { column, .. } if column == "age"));
}

#[test]
fn column_change_default_variant() {
  let change = ColumnChange::Default {
    column: "status".to_owned(),
    old: None,
    new: Some("'draft'".to_owned()),
  };
  assert!(
    matches!(change, ColumnChange::Default { column, ref new, .. } if column == "status" && new == &Some("'draft'".to_owned()))
  );
}

#[test]
fn column_change_nullable_variant() {
  let change = ColumnChange::Nullable {
    column: "email".to_owned(),
    old: true,
    new: false,
  };
  assert!(matches!(change, ColumnChange::Nullable { old, new, .. } if old && !new));
}

#[test]
fn column_change_unique_variant() {
  let change = ColumnChange::Unique {
    column: "email".to_owned(),
    old: false,
    new: true,
  };
  assert!(matches!(change, ColumnChange::Unique { old, new, .. } if !old && new));
}

#[test]
fn operation_create_enum() {
  let op = Operation::CreateEnum {
    name: "status".to_owned(),
    variants: vec!["draft".to_owned(), "active".to_owned()],
  };
  assert!(
    matches!(op, Operation::CreateEnum { ref name, ref variants } if name == "status" && variants.len() == 2)
  );
}

#[test]
fn operation_rename_table() {
  let op = Operation::RenameTable {
    old: "users".to_owned(),
    new: "accounts".to_owned(),
  };
  assert!(
    matches!(op, Operation::RenameTable { ref old, ref new } if old == "users" && new == "accounts")
  );
}

#[test]
fn operation_rename_column() {
  let op = Operation::RenameColumn {
    table: "users".to_owned(),
    old: "name".to_owned(),
    new: "full_name".to_owned(),
  };
  assert!(
    matches!(op, Operation::RenameColumn { ref table, ref old, ref new } if table == "users" && old == "name" && new == "full_name")
  );
}

#[test]
fn operation_alter_column_batched() {
  let table_def = table("users", vec![]);
  let op = Operation::AlterColumn {
    table: "users".to_owned(),
    changes: vec![
      ColumnChange::Type {
        column: "age".to_owned(),
        old: ColumnType::Integer,
        new: ColumnType::BigInt,
      },
      ColumnChange::Nullable {
        column: "age".to_owned(),
        old: true,
        new: false,
      },
    ],
    table_def,
  };
  assert!(
    matches!(
      &op,
      Operation::AlterColumn { ref changes, .. } if changes.len() == 2
    ),
    "expected AlterColumn with two batched changes, got {op:?}"
  );
}

#[test]
fn operation_add_foreign_key() {
  use toolu_orm_core::column::ForeignKeyAction;
  use toolu_orm_core::snapshot::ForeignKeyDef;
  let fk = ForeignKeyDef {
    name: "fk_posts_author".to_owned(),
    columns: vec!["author_id".to_owned()],
    references_table: "users".to_owned(),
    references_columns: vec!["id".to_owned()],
    on_delete: Some(ForeignKeyAction::Cascade),
    on_update: None,
  };
  let op = Operation::AddForeignKey {
    table: "posts".to_owned(),
    fk,
  };
  assert!(matches!(op, Operation::AddForeignKey { ref table, .. } if table == "posts"));
}

#[test]
fn operation_add_check_constraint() {
  let op = Operation::AddCheckConstraint {
    table: "items".to_owned(),
    name: "chk_status".to_owned(),
    expr: "CHECK(\"status\" IN ('draft', 'active'))".to_owned(),
  };
  assert!(
    matches!(op, Operation::AddCheckConstraint { ref table, ref name, .. } if table == "items" && name == "chk_status")
  );
}
