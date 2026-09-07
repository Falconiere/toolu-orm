use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::diff::{ColumnChange, Operation};
use toolu_orm_core::ordering::order_operations;
use toolu_orm_core::table::TableDef;

#[test]
fn full_ordering_13_tiers() {
  let ops = vec![
    Operation::DropEnum {
      name: "old_enum".to_owned(),
    },
    Operation::DropTable {
      name: "old_table".to_owned(),
    },
    Operation::DropColumn {
      table: "t".to_owned(),
      column: "c".to_owned(),
    },
    Operation::AddCheckConstraint {
      table: "t".to_owned(),
      name: "chk".to_owned(),
      expr: "expr".to_owned(),
    },
    Operation::AddColumn {
      table: "t".to_owned(),
      column: ColumnDef {
        name: "new_col".to_owned(),
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
    },
    Operation::AlterColumn {
      table: "t".to_owned(),
      changes: vec![ColumnChange::Nullable {
        column: "x".to_owned(),
        old: true,
        new: false,
      }],
      table_def: TableDef {
        name: "t".to_owned(),
        columns: vec![],
        indexes: vec![],
        strict: false,
      },
    },
    Operation::DropCheckConstraint {
      table: "t".to_owned(),
      name: "old_chk".to_owned(),
    },
    Operation::RenameColumn {
      table: "t".to_owned(),
      old: "a".to_owned(),
      new: "b".to_owned(),
    },
    Operation::RenameTable {
      old: "x".to_owned(),
      new: "y".to_owned(),
    },
    Operation::CreateTable {
      table: TableDef {
        name: "new_table".to_owned(),
        columns: vec![],
        indexes: vec![],
        strict: false,
      },
    },
    Operation::AlterEnum {
      name: "e".to_owned(),
      added: vec!["v".to_owned()],
      removed: vec![],
    },
    Operation::CreateEnum {
      name: "new_enum".to_owned(),
      variants: vec!["a".to_owned()],
    },
    Operation::AlterEnum {
      name: "e2".to_owned(),
      added: vec![],
      removed: vec!["old_v".to_owned()],
    },
  ];

  let ordered = order_operations(ops);

  let tier = |op: &Operation| -> u8 {
    match op {
      Operation::CreateEnum { .. } => 1,
      Operation::AlterEnum { removed, .. } if removed.is_empty() => 2,
      Operation::CreateTable { .. } => 3,
      Operation::RenameTable { .. } => 4,
      Operation::RenameColumn { .. } => 5,
      Operation::DropForeignKey { .. }
      | Operation::DropIndex { .. }
      | Operation::DropCheckConstraint { .. } => 6,
      Operation::AlterColumn { .. } => 7,
      Operation::AddColumn { .. } => 8,
      Operation::AddForeignKey { .. }
      | Operation::CreateIndex { .. }
      | Operation::AddCheckConstraint { .. } => 9,
      Operation::DropColumn { .. } => 10,
      Operation::DropTable { .. } => 11,
      Operation::DropEnum { .. } => 12,
      Operation::AlterEnum { .. } => 13,
    }
  };

  for window in ordered.windows(2) {
    match window {
      [a, b] => assert!(
        tier(a) <= tier(b),
        "ordering violation: {:?} (tier {}) came before {:?} (tier {})",
        a,
        tier(a),
        b,
        tier(b)
      ),
      _ => assert_eq!(window.len(), 2, "windows(2) must yield length-2 slices"),
    }
  }
}
