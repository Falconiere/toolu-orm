//! SQL generation from a list of migration operations for both dialects.

use crate::dialect::Dialect;
use crate::diff::Operation;
use crate::ordering::order_operations;

use super::foreign_key::add_constraint_sql;
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
/// SQLite cannot alter in place into one table rebuild. On Postgres every
/// foreign-key constraint — an `AddForeignKey`, or a table-level key of a
/// created table — comes last: Postgres checks the referenced unique key when
/// the constraint is created, and the indexes it needs come after the tables.
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
      let mut foreign_keys: Vec<String> = Vec::new();
      for op in ordered {
        match &op {
          Operation::AddForeignKey { .. } => {
            foreign_keys.push(operation_sql(&op, dialect));
            continue;
          },
          Operation::CreateTable { table } => foreign_keys.extend(
            table
              .foreign_keys
              .iter()
              .map(|fk| add_constraint_sql(&table.name, fk)),
          ),
          _ => {},
        }
        push_chunk(&mut parts, operation_sql(&op, dialect));
      }
      for chunk in foreign_keys {
        push_chunk(&mut parts, chunk);
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
