//! Operation ordering for migrations using a 13-tier priority system.

use crate::diff::Operation;

/// Sorts operations into dependency-safe execution order.
pub fn order_operations(ops: Vec<Operation>) -> Vec<Operation> {
  let mut indexed: Vec<(usize, Operation)> = ops.into_iter().enumerate().collect();
  indexed.sort_by_key(|(i, op)| (priority(op), *i));
  indexed.into_iter().map(|(_, op)| op).collect()
}

fn priority(op: &Operation) -> u8 {
  match op {
    Operation::CreateEnum { .. } => 1,
    Operation::AlterEnum { removed, .. } if removed.is_empty() => 2,
    Operation::CreateTable { .. } => 3,
    Operation::RenameTable { .. } => 4,
    Operation::RenameColumn { .. } => 5,
    Operation::DropForeignKey { .. }
    | Operation::DropIndex { .. }
    | Operation::DropCheckConstraint { .. } => 6,
    Operation::AlterColumn { .. } | Operation::RecreateFts5FromContent { .. } => 7,
    Operation::AddColumn { .. } => 8,
    Operation::AddForeignKey { .. }
    | Operation::CreateIndex { .. }
    | Operation::AddCheckConstraint { .. } => 9,
    Operation::DropColumn { .. } => 10,
    Operation::DropTable { .. } => 11,
    Operation::DropEnum { .. } => 12,
    Operation::AlterEnum { .. } => 13,
  }
}
