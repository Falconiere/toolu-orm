//! Postgres-specific ALTER column and unique constraint statements.

use crate::dialect::Dialect;
use crate::diff::ColumnChange;
use crate::table::TableDef;

use super::translate::translate_default;

/// Name of the index Postgres uses to carry a column-level `UNIQUE`.
pub(crate) fn unique_index_name(table: &str, column: &str) -> String {
  format!("uq_{table}_{column}")
}

/// One `ALTER TABLE` per column change; Postgres alters in place, so no
/// change here needs a table rebuild.
pub(crate) fn alter_column_statements_postgres(
  table: &str,
  changes: &[ColumnChange],
  _table_def: &TableDef,
) -> Vec<String> {
  changes
    .iter()
    .filter_map(|ch| alter_column_statement(table, ch))
    .collect()
}

/// The statement for one change, or `None` when the change is already in the
/// requested state (a `UNIQUE` flag that did not actually flip).
fn alter_column_statement(table: &str, change: &ColumnChange) -> Option<String> {
  match change {
    ColumnChange::Type { column, new, .. } => {
      let ty = new.as_ddl_sql(Dialect::Postgres);
      Some(format!(
        "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" TYPE {ty};"
      ))
    },
    ColumnChange::Default { column, new, .. } => Some(match new {
      Some(d) => {
        let td = translate_default(d, Dialect::Postgres);
        format!("ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET DEFAULT ({td});")
      },
      None => format!("ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP DEFAULT;"),
    }),
    ColumnChange::Nullable { column, new, .. } => Some(if *new {
      format!("ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP NOT NULL;")
    } else {
      format!("ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET NOT NULL;")
    }),
    ColumnChange::Unique { column, old, new } => unique_statement(table, column, *old, *new),
    ColumnChange::PrimaryKey { column, .. } | ColumnChange::Autoincrement { column, .. } => {
      Some(format!(
        "-- PRIMARY KEY / AUTOINCREMENT change on \"{table}\".\"{column}\" requires table recreation on Postgres"
      ))
    },
    ColumnChange::CompositePrimaryKey { .. } => Some(format!(
      "-- composite PRIMARY KEY change on \"{table}\" requires table recreation on Postgres"
    )),
  }
}

/// Creates or drops the backing unique index, or nothing when the flag is
/// unchanged.
fn unique_statement(table: &str, column: &str, old: bool, new: bool) -> Option<String> {
  let idx = unique_index_name(table, column);
  match (old, new) {
    (true, false) => Some(format!("DROP INDEX IF EXISTS \"{idx}\";")),
    (false, true) => Some(format!(
      "CREATE UNIQUE INDEX IF NOT EXISTS \"{idx}\" ON \"{table}\" (\"{column}\");"
    )),
    (false, false) | (true, true) => None,
  }
}
