//! SQL generation from a list of migration operations for both dialects.

use crate::dialect::Dialect;
use crate::diff::Operation;
use crate::ordering::order_operations;

use super::operation_sql::operation_sql;
use super::rebuild::{plan_sqlite_rebuilds, rebuild_sql, SqliteStep};

/// Migration SQL for the dialect this build targets.
pub fn generate_sql(operations: &[Operation]) -> String {
  generate_sql_for(operations, Dialect::CURRENT)
}

/// Migration SQL for `dialect`, as `--> statement-breakpoint`-separated chunks.
///
/// On SQLite the ordered operations first pass through
/// `plan_sqlite_rebuilds`, which folds every column operation on a table that
/// SQLite cannot alter in place into one table rebuild.
pub fn generate_sql_for(operations: &[Operation], dialect: Dialect) -> String {
  let ordered = order_operations(operations.to_vec());
  let mut parts: Vec<String> = Vec::new();
  match dialect {
    Dialect::Sqlite => {
      for step in plan_sqlite_rebuilds(ordered) {
        push_chunk(&mut parts, sqlite_step_sql(&step));
      }
    },
    Dialect::Postgres => {
      for op in ordered {
        push_chunk(&mut parts, operation_sql(&op, dialect));
      }
    },
    Dialect::Lance => {
      if !ordered.is_empty() {
        parts.push("-- Lance migration SQL is unsupported; see issue #181".to_owned());
      }
    },
  }
  parts.join("\n\n--> statement-breakpoint\n\n")
}

/// Keeps a chunk unless the operation rendered nothing at all.
fn push_chunk(parts: &mut Vec<String>, chunk: String) {
  if !chunk.trim().is_empty() {
    parts.push(chunk);
  }
}

/// SQL for one planned SQLite step.
fn sqlite_step_sql(step: &SqliteStep) -> String {
  match step {
    SqliteStep::Operation(op) => operation_sql(op, Dialect::Sqlite),
    SqliteStep::Rebuild(plan) => rebuild_sql(plan),
  }
}
