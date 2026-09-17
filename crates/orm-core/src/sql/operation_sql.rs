//! SQL for one migration operation, in either dialect.

use crate::dialect::Dialect;
use crate::diff::{ColumnChange, Operation};
use crate::snapshot::ForeignKeyDef;
use crate::table::TableDef;

use super::ddl::{
  add_column_sql, create_index_sql, create_table_sql, recreate_fts5_from_content_sql,
};
use super::fts5_triggers::{create_sync_triggers_sql, drop_sync_triggers_sql};
use super::postgres::alter_column_statements_postgres;

/// Separator between several statements rendered for one operation.
const BREAKPOINT: &str = "\n\n--> statement-breakpoint\n\n";

/// One operation as SQL. An operation a dialect cannot express renders as a
/// `--` comment, and an operation the SQLite rebuild planner already absorbed
/// renders as the empty string.
pub(super) fn operation_sql(op: &Operation, dialect: Dialect) -> String {
  match op {
    Operation::CreateEnum { name, variants } => create_enum_sql(name, variants, dialect),
    Operation::AlterEnum {
      name,
      added,
      removed,
    } => alter_enum_sql(name, added, removed, dialect),
    Operation::DropEnum { name } => drop_enum_sql(name, dialect),
    Operation::CreateTable { table } => create_table_sql(table, dialect),
    Operation::DropTable { name } => format!("DROP TABLE IF EXISTS \"{name}\";"),
    Operation::RenameTable { old, new } => format!("ALTER TABLE \"{old}\" RENAME TO \"{new}\";"),
    Operation::RenameColumn { table, old, new } => {
      format!("ALTER TABLE \"{table}\" RENAME COLUMN \"{old}\" TO \"{new}\";")
    },
    Operation::AddColumn { table, column } => add_column_sql(table, column, dialect),
    Operation::DropColumn { table, column } => drop_column_sql(table, column, dialect),
    Operation::AlterColumn {
      table,
      changes,
      table_def,
    } => alter_column_sql(table, changes, table_def, dialect),
    Operation::CreateIndex { table, index } => create_index_sql(table, index),
    Operation::DropIndex { name } => format!("DROP INDEX IF EXISTS \"{name}\";"),
    Operation::AddForeignKey { table, fk } => add_foreign_key_sql(table, fk, dialect),
    Operation::DropForeignKey { table, name } => drop_foreign_key_sql(table, name, dialect),
    Operation::AddCheckConstraint { table, name, expr } => {
      add_check_constraint_sql(table, name, expr, dialect)
    },
    Operation::DropCheckConstraint { table, name } => {
      drop_check_constraint_sql(table, name, dialect)
    },
    Operation::RecreateFts5FromContent { table } => recreate_fts5_from_content_sql(table, dialect),
    Operation::DropFts5SyncTriggers { table } => drop_sync_triggers_sql(table, dialect),
    Operation::CreateFts5SyncTriggers { table, sync } => {
      create_sync_triggers_sql(table, sync, dialect)
    },
  }
}

/// SQLite has no enum type, so dropping one changes nothing there.
fn drop_enum_sql(name: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => format!("DROP TYPE IF EXISTS \"{name}\";"),
    Dialect::Sqlite => format!("-- drop enum \"{name}\" (no-op on SQLite)"),
  }
}

/// SQLite's `DROP COLUMN` has no `IF EXISTS`.
fn drop_column_sql(table: &str, column: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => format!("ALTER TABLE \"{table}\" DROP COLUMN IF EXISTS \"{column}\";"),
    Dialect::Sqlite => format!("ALTER TABLE \"{table}\" DROP COLUMN \"{column}\";"),
  }
}

/// SQLite can only detach a foreign key by rebuilding the table.
fn drop_foreign_key_sql(table: &str, name: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => format!("ALTER TABLE \"{table}\" DROP CONSTRAINT IF EXISTS \"{name}\";"),
    Dialect::Sqlite => {
      format!("-- drop FOREIGN KEY \"{name}\" on \"{table}\" (SQLite: rebuild table)")
    },
  }
}

/// SQLite cannot add a CHECK to an existing table without rebuilding it.
fn add_check_constraint_sql(table: &str, name: &str, expr: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => {
      format!("ALTER TABLE \"{table}\" ADD CONSTRAINT \"{name}\" CHECK ({expr});")
    },
    Dialect::Sqlite => {
      format!("-- ADD CHECK \"{name}\" on \"{table}\" ({expr}) — SQLite may require table rebuild")
    },
  }
}

/// SQLite cannot drop a CHECK from an existing table without rebuilding it.
fn drop_check_constraint_sql(table: &str, name: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => format!("ALTER TABLE \"{table}\" DROP CONSTRAINT IF EXISTS \"{name}\";"),
    Dialect::Sqlite => {
      format!("-- DROP CHECK \"{name}\" on \"{table}\" (SQLite may require table rebuild)")
    },
  }
}

/// `CREATE TYPE … AS ENUM` on Postgres; SQLite stores enums as TEXT + CHECK.
fn create_enum_sql(name: &str, variants: &[String], dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => {
      let vals = variants
        .iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(", ");
      format!("CREATE TYPE \"{name}\" AS ENUM ({vals});")
    },
    Dialect::Sqlite => format!("-- enum \"{name}\" (SQLite: TEXT + CHECK)"),
  }
}

/// Added variants are a plain `ADD VALUE` on Postgres; removal needs a type
/// recreation there, and either direction is a CHECK edit on SQLite.
fn alter_enum_sql(name: &str, added: &[String], removed: &[String], dialect: Dialect) -> String {
  let mut s = String::new();
  if !added.is_empty() {
    match dialect {
      Dialect::Postgres => {
        let stmts: Vec<String> = added
          .iter()
          .map(|v| format!("ALTER TYPE \"{name}\" ADD VALUE '{v}';"))
          .collect();
        s.push_str(&stmts.join(BREAKPOINT));
      },
      Dialect::Sqlite => {
        s.push_str(&format!(
          "-- ALTER ENUM \"{name}\" add variants (SQLite: adjust CHECK)\n"
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
      Dialect::Sqlite => s.push_str("-- SQLite: adjust CHECK for enum variant removal\n"),
    }
  }
  s.trim_end().to_owned()
}

/// Postgres alters in place. On SQLite the change is applied by a table
/// rebuild, which [`plan_sqlite_rebuilds`](super::rebuild::plan_sqlite_rebuilds)
/// has already emitted, so nothing is left to render here.
fn alter_column_sql(
  table: &str,
  changes: &[ColumnChange],
  table_def: &TableDef,
  dialect: Dialect,
) -> String {
  if changes.is_empty() {
    return String::new();
  }
  match dialect {
    Dialect::Postgres => {
      alter_column_statements_postgres(table, changes, table_def).join(BREAKPOINT)
    },
    Dialect::Sqlite => String::new(),
  }
}

/// `ADD CONSTRAINT … FOREIGN KEY` on Postgres; SQLite can only attach a
/// constraint by rebuilding the table.
fn add_foreign_key_sql(table: &str, fk: &ForeignKeyDef, dialect: Dialect) -> String {
  match dialect {
    Dialect::Sqlite => format!(
      "-- FOREIGN KEY \"{}\" on \"{table}\" (SQLite: rebuild table to attach constraint)",
      fk.name
    ),
    Dialect::Postgres => {
      let cols = quoted_csv(&fk.columns);
      let ref_cols = quoted_csv(&fk.references_columns);
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
  }
}

/// `"a", "b"` — identifiers quoted and comma-joined.
fn quoted_csv(names: &[String]) -> String {
  names
    .iter()
    .map(|c| format!("\"{c}\""))
    .collect::<Vec<_>>()
    .join(", ")
}
