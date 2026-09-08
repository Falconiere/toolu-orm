//! DDL helpers for table, column, and index creation statements.

use crate::column::ColumnDef;
use crate::dialect::Dialect;
use crate::index::IndexDef;
use crate::table::{TableDef, TableKind};

use super::translate::translate_default;
use super::virtual_table::{create_virtual_table_sql, unsupported_dialect_comment};

pub(crate) fn format_references(refs: &str) -> String {
  let Some((table, col_with_paren)) = refs.split_once('(') else {
    return format!("REFERENCES {refs}");
  };
  let column = col_with_paren.trim_end_matches(')');
  format!("REFERENCES \"{table}\"(\"{column}\")")
}

pub(crate) fn column_def_sql(col: &ColumnDef, strict: bool, dialect: Dialect) -> String {
  let type_sql = if matches!(dialect, Dialect::Sqlite) && strict {
    col.column_type.as_sql()
  } else {
    col.column_type.as_ddl_sql(dialect)
  };
  let mut parts = vec![format!("\"{}\" {}", col.name, type_sql)];

  if col.not_null {
    parts.push("NOT NULL".to_owned());
  }
  if col.primary_key {
    parts.push("PRIMARY KEY".to_owned());
  }
  if col.unique {
    parts.push("UNIQUE".to_owned());
  }
  if let Some(default) = &col.default {
    let d = translate_default(default, dialect);
    parts.push(format!("DEFAULT ({d})"));
  }
  if let Some(refs) = &col.references {
    let mut refs_part = format_references(refs);
    if let Some(on_delete) = &col.on_delete {
      refs_part.push_str(&format!(" ON DELETE {}", on_delete.as_sql()));
    }
    if let Some(on_update) = &col.on_update {
      refs_part.push_str(&format!(" ON UPDATE {}", on_update.as_sql()));
    }
    parts.push(refs_part);
  }
  if let Some(check) = &col.check {
    parts.push(check.clone());
  }

  parts.join(" ")
}

pub(crate) fn create_table_sql(table: &TableDef, dialect: Dialect) -> String {
  if let TableKind::Virtual { module, args } = &table.kind {
    return match dialect {
      Dialect::Sqlite => create_virtual_table_sql(&table.name, module, args),
      Dialect::Postgres => unsupported_dialect_comment(&table.name, module, dialect),
    };
  }
  let col_defs: Vec<String> = table
    .columns
    .iter()
    .map(|c| column_def_sql(c, table.strict, dialect))
    .collect();
  let cols = col_defs.join(",\n    ");
  if table.strict && matches!(dialect, Dialect::Sqlite) {
    format!(
      "CREATE TABLE IF NOT EXISTS \"{}\" (\n    {cols}\n) STRICT;",
      table.name
    )
  } else {
    format!(
      "CREATE TABLE IF NOT EXISTS \"{}\" (\n    {cols}\n);",
      table.name
    )
  }
}

pub(crate) fn create_index_sql(table: &str, index: &IndexDef) -> String {
  let unique = if index.unique { "UNIQUE " } else { "" };
  let cols: Vec<String> = index.columns.iter().map(|c| format!("\"{c}\"")).collect();
  let cols_str = cols.join(", ");
  format!(
    "CREATE {unique}INDEX IF NOT EXISTS \"{}\" ON \"{table}\" ({cols_str});",
    index.name
  )
}

pub(crate) fn add_column_sql(table: &str, column: &ColumnDef, dialect: Dialect) -> String {
  let col_sql = column_def_sql(column, false, dialect);
  format!("ALTER TABLE \"{table}\" ADD COLUMN {col_sql};")
}

pub(crate) fn recreation_sql(table_name: &str, new_def: &TableDef) -> String {
  let old_name = format!("_{table_name}_old");
  let cols_csv = new_def
    .columns
    .iter()
    .map(|c| format!("\"{}\"", c.name))
    .collect::<Vec<_>>()
    .join(", ");
  let create_sql = create_table_sql(new_def, Dialect::Sqlite);

  format!(
    "PRAGMA foreign_keys = OFF;\n\
     --> statement-breakpoint\n\
     ALTER TABLE \"{table_name}\" RENAME TO \"{old_name}\";\n\
     --> statement-breakpoint\n\
     {create_sql}\n\
     --> statement-breakpoint\n\
     INSERT INTO \"{table_name}\" ({cols_csv}) SELECT {cols_csv} FROM \"{old_name}\";\n\
     --> statement-breakpoint\n\
     DROP TABLE \"{old_name}\";\n\
     --> statement-breakpoint\n\
     PRAGMA foreign_keys = ON;"
  )
}
