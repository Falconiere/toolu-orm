//! The statement sequence for one SQLite table rebuild.

use crate::dialect::Dialect;
use crate::sql::ddl::{create_index_sql, create_table_sql_named};

use super::plan::RebuildPlan;

/// Separator between the rebuild's own statements, matching the one
/// [`generate_sql_for`](crate::sql::generate_sql_for) puts between chunks.
const BREAKPOINT: &str = "\n--> statement-breakpoint\n";

/// The name the rebuild creates the replacement table under before renaming it
/// into place. Prefixed so it cannot collide with a declared table by accident.
fn staging_table_name(table: &str) -> String {
  format!("_toolu_new_{table}")
}

/// Renders SQLite's documented rebuild: create the staging table, copy the
/// surviving columns, drop the old table, rename the staging table into place,
/// then re-create every declared index.
///
/// The leading `PRAGMA foreign_keys = OFF` is a request to the migration
/// runner, which applies it outside the transaction and restores the caller's
/// original setting afterwards; inside a transaction SQLite documents the
/// pragma as a no-op.
pub(crate) fn rebuild_sql(plan: &RebuildPlan) -> String {
  let name = &plan.table.name;
  let staging = staging_table_name(name);
  let mut parts = vec![
    format!(
      "-- Rebuild \"{name}\": SQLite cannot alter these columns in place.\n\
       -- The runner hoists this pragma outside the transaction, where it is\n\
       -- not a no-op, and restores the original setting afterwards.\n\
       PRAGMA foreign_keys = OFF;"
    ),
    create_table_sql_named(&staging, &plan.table, Dialect::Sqlite, false),
  ];
  if !plan.copy_columns.is_empty() {
    let cols = plan
      .copy_columns
      .iter()
      .map(|c| format!("\"{c}\""))
      .collect::<Vec<_>>()
      .join(", ");
    parts.push(format!(
      "INSERT INTO \"{staging}\" ({cols}) SELECT {cols} FROM \"{name}\";"
    ));
  }
  parts.push(format!("DROP TABLE \"{name}\";"));
  // Without this, SQLite re-parses every view and trigger while renaming and
  // fails on any that still names the table this statement is about to restore
  // ("error in view …: no such table"). Legacy mode also keeps the rename from
  // rewriting references, which is what a rebuild wants: nothing points at the
  // staging name, and everything that points at "{name}" is already correct.
  parts.push("PRAGMA legacy_alter_table = ON;".to_owned());
  parts.push(format!("ALTER TABLE \"{staging}\" RENAME TO \"{name}\";"));
  parts.push("PRAGMA legacy_alter_table = OFF;".to_owned());
  for index in &plan.table.indexes {
    parts.push(create_index_sql(name, index));
  }
  parts.join(BREAKPOINT)
}
