//! Postgres-specific ALTER column and unique constraint statements.

use crate::dialect::Dialect;
use crate::diff::ColumnChange;
use crate::table::TableDef;

use super::translate::translate_default;

pub(crate) fn unique_index_name(table: &str, column: &str) -> String {
  format!("uq_{table}_{column}")
}

pub(crate) fn alter_column_statements_postgres(
  table: &str,
  changes: &[ColumnChange],
  _table_def: &TableDef,
) -> Vec<String> {
  let mut out = Vec::new();
  for ch in changes {
    match ch {
      ColumnChange::Type { column, new, .. } => {
        let ty = new.as_ddl_sql(Dialect::Postgres);
        out.push(format!(
          "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" TYPE {ty};"
        ));
      },
      ColumnChange::Default { column, new, .. } => {
        if let Some(d) = new {
          let td = translate_default(d, Dialect::Postgres);
          out.push(format!(
            "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET DEFAULT ({td});"
          ));
        } else {
          out.push(format!(
            "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP DEFAULT;"
          ));
        }
      },
      ColumnChange::Nullable { column, new, .. } => {
        if *new {
          out.push(format!(
            "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP NOT NULL;"
          ));
        } else {
          out.push(format!(
            "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET NOT NULL;"
          ));
        }
      },
      ColumnChange::Unique { column, old, new } => {
        let idx = unique_index_name(table, column);
        if *old && !*new {
          out.push(format!("DROP INDEX IF EXISTS \"{idx}\";"));
        } else if !old && *new {
          out.push(format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS \"{idx}\" ON \"{table}\" (\"{column}\");"
          ));
        }
      },
    }
  }
  out
}

pub(crate) fn needs_recreation_sqlite(changes: &[ColumnChange]) -> bool {
  changes.iter().any(|c| {
    matches!(
      c,
      ColumnChange::Type { .. }
        | ColumnChange::Default { .. }
        | ColumnChange::Nullable { .. }
        | ColumnChange::Unique { .. }
    )
  })
}
