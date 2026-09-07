//! SQL generation from a list of migration operations for both dialects.

use crate::dialect::Dialect;
use crate::diff::Operation;
use crate::ordering::order_operations;

use super::ddl::{add_column_sql, create_index_sql, create_table_sql, recreation_sql};
use super::postgres::{alter_column_statements_postgres, needs_recreation_sqlite};

pub fn generate_sql(operations: &[Operation]) -> String {
  generate_sql_for(operations, Dialect::CURRENT)
}

pub fn generate_sql_for(operations: &[Operation], dialect: Dialect) -> String {
  let ordered = order_operations(operations.to_vec());
  let mut parts: Vec<String> = Vec::new();
  for op in ordered {
    let chunk = operation_sql(&op, dialect);
    if !chunk.trim().is_empty() {
      parts.push(chunk);
    }
  }
  parts.join("\n\n--> statement-breakpoint\n\n")
}

fn operation_sql(op: &Operation, dialect: Dialect) -> String {
  match op {
    Operation::CreateEnum { name, variants } => match dialect {
      Dialect::Postgres => {
        let vals = variants
          .iter()
          .map(|v| format!("'{v}'"))
          .collect::<Vec<_>>()
          .join(", ");
        format!("CREATE TYPE \"{name}\" AS ENUM ({vals});")
      },
      Dialect::Sqlite => format!("-- enum \"{name}\" (SQLite: TEXT + CHECK)"),
    },
    Operation::AlterEnum {
      name,
      added,
      removed,
    } => {
      let mut s = String::new();
      if !added.is_empty() {
        match dialect {
          Dialect::Postgres => {
            let stmts: Vec<String> = added
              .iter()
              .map(|v| format!("ALTER TYPE \"{name}\" ADD VALUE '{v}';"))
              .collect();
            s.push_str(&stmts.join("\n\n--> statement-breakpoint\n\n"));
          },
          Dialect::Sqlite => {
            s.push_str(&format!(
              "-- ALTER ENUM \"{name}\" add variants (SQLite: adjust CHECK)\n",
            ));
          },
        }
      }
      if !removed.is_empty() {
        match dialect {
          Dialect::Postgres => {
            s.push_str("-- TODO: enum variant removal requires type recreation on Postgres for ");
            s.push_str(name);
            s.push('\n');
          },
          Dialect::Sqlite => {
            s.push_str("-- SQLite: adjust CHECK for enum variant removal\n");
          },
        }
      }
      s.trim_end().to_owned()
    },
    Operation::DropEnum { name } => match dialect {
      Dialect::Postgres => format!("DROP TYPE IF EXISTS \"{name}\";"),
      Dialect::Sqlite => format!("-- drop enum \"{name}\" (no-op on SQLite)"),
    },
    Operation::CreateTable { table } => create_table_sql(table, dialect),
    Operation::DropTable { name } => format!("DROP TABLE IF EXISTS \"{name}\";"),
    Operation::RenameTable { old, new } => {
      format!("ALTER TABLE \"{old}\" RENAME TO \"{new}\";")
    },
    Operation::RenameColumn { table, old, new } => {
      format!("ALTER TABLE \"{table}\" RENAME COLUMN \"{old}\" TO \"{new}\";")
    },
    Operation::AddColumn { table, column } => add_column_sql(table, column, dialect),
    Operation::DropColumn { table, column } => match dialect {
      Dialect::Postgres => format!("ALTER TABLE \"{table}\" DROP COLUMN IF EXISTS \"{column}\";"),
      Dialect::Sqlite => format!("ALTER TABLE \"{table}\" DROP COLUMN \"{column}\";"),
    },
    Operation::AlterColumn {
      table,
      changes,
      table_def,
    } => {
      if changes.is_empty() {
        return String::new();
      }
      match dialect {
        Dialect::Postgres => alter_column_statements_postgres(table, changes, table_def)
          .join("\n\n--> statement-breakpoint\n\n"),
        Dialect::Sqlite => {
          if needs_recreation_sqlite(changes) {
            recreation_sql(table, table_def)
          } else {
            String::new()
          }
        },
      }
    },
    Operation::CreateIndex { table, index } => create_index_sql(table, index),
    Operation::DropIndex { name } => format!("DROP INDEX IF EXISTS \"{name}\";"),
    Operation::AddForeignKey { table, fk } => match dialect {
      Dialect::Postgres => {
        let cols = fk
          .columns
          .iter()
          .map(|c| format!("\"{c}\""))
          .collect::<Vec<_>>()
          .join(", ");
        let ref_cols = fk
          .references_columns
          .iter()
          .map(|c| format!("\"{c}\""))
          .collect::<Vec<_>>()
          .join(", ");
        let mut s = format!(
          "ALTER TABLE \"{table}\" ADD CONSTRAINT \"{}\" FOREIGN KEY ({cols}) REFERENCES \"{}\" ({ref_cols})",
          fk.name, fk.references_table
        );
        if let Some(a) = fk.on_delete {
          s.push_str(&format!(" ON DELETE {}", a.as_sql()));
        }
        if let Some(a) = fk.on_update {
          s.push_str(&format!(" ON UPDATE {}", a.as_sql()));
        }
        s.push(';');
        s
      },
      Dialect::Sqlite => format!(
        "-- FOREIGN KEY \"{}\" on \"{table}\" (SQLite: rebuild table to attach constraint)",
        fk.name
      ),
    },
    Operation::DropForeignKey { table, name } => match dialect {
      Dialect::Postgres => {
        format!("ALTER TABLE \"{table}\" DROP CONSTRAINT IF EXISTS \"{name}\";")
      },
      Dialect::Sqlite => {
        format!("-- drop FOREIGN KEY \"{name}\" on \"{table}\" (SQLite: rebuild table)")
      },
    },
    Operation::AddCheckConstraint { table, name, expr } => match dialect {
      Dialect::Postgres => {
        format!("ALTER TABLE \"{table}\" ADD CONSTRAINT \"{name}\" CHECK ({expr});")
      },
      Dialect::Sqlite => format!(
        "-- ADD CHECK \"{name}\" on \"{table}\" ({expr}) — SQLite may require table rebuild"
      ),
    },
    Operation::DropCheckConstraint { table, name } => match dialect {
      Dialect::Postgres => {
        format!("ALTER TABLE \"{table}\" DROP CONSTRAINT IF EXISTS \"{name}\";")
      },
      Dialect::Sqlite => {
        format!("-- DROP CHECK \"{name}\" on \"{table}\" (SQLite may require table rebuild)")
      },
    },
  }
}
