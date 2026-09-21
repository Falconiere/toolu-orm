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
    // FTS5 synchronization triggers come down with the other drops: before an
    // FTS table is recreated, and before a content-table rebuild would take
    // them with its `DROP TABLE`.
    Operation::DropForeignKey { .. }
    | Operation::DropIndex { .. }
    | Operation::DropCheckConstraint { .. }
    | Operation::DropFts5SyncTriggers { .. }
    | Operation::DropPolicy { .. } => 6,
    Operation::AlterColumn { .. } | Operation::RecreateFts5FromContent { .. } => 7,
    Operation::AddColumn { .. } => 8,
    // And go back up with the other creates, once both tables are in place.
    Operation::AddForeignKey { .. }
    | Operation::CreateIndex { .. }
    | Operation::AddCheckConstraint { .. }
    | Operation::CreateFts5SyncTriggers { .. }
    | Operation::AlterRowLevelSecurity { .. }
    | Operation::CreatePolicy { .. } => 9,
    Operation::DropColumn { .. } => 10,
    Operation::DropTable { .. } => 11,
    Operation::DropEnum { .. } => 12,
    Operation::AlterEnum { .. } => 13,
  }
}
