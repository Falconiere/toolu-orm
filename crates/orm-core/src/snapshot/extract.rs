//! Foreign key and constraint extraction from column definitions into snapshot format.

use std::collections::BTreeMap;

use crate::column::ColumnDef;
use crate::table::TableDef;

use super::types::ForeignKeyDef;

/// The name a `#[column(references = …)]` foreign key is stored under.
fn column_fk_name(table_name: &str, column: &str) -> String {
  format!("fk_{table_name}_{column}")
}

/// True when `fk` is the one a column's own `references` produced: a single
/// column, stored under that column's name, on a column the table declares.
/// Anything else is a table-level foreign key.
pub(crate) fn is_column_derived<'a>(
  table_name: &str,
  fk: &ForeignKeyDef,
  mut columns: impl Iterator<Item = &'a String>,
) -> bool {
  let ([column], [_]) = (fk.columns.as_slice(), fk.references_columns.as_slice()) else {
    return false;
  };
  fk.name == column_fk_name(table_name, column) && columns.any(|c| c == column)
}

/// The table-level foreign keys of a snapshot table, keyed by name.
pub(crate) fn table_level_foreign_keys(
  table_name: &str,
  table: &super::types::SnapshotTable,
) -> BTreeMap<String, ForeignKeyDef> {
  table
    .foreign_keys
    .iter()
    .filter(|(_, fk)| !is_column_derived(table_name, fk, table.columns.keys()))
    .map(|(name, fk)| (name.clone(), fk.clone()))
    .collect()
}

/// Every foreign key of `table`, per-column and table-level, keyed by name.
pub(crate) fn extract_foreign_keys(table: &TableDef) -> BTreeMap<String, ForeignKeyDef> {
  let mut fks = extract_column_foreign_keys(&table.name, &table.columns);
  for fk in &table.foreign_keys {
    fks.insert(fk.name.clone(), fk.clone());
  }
  fks
}

fn extract_column_foreign_keys(
  table_name: &str,
  columns: &[ColumnDef],
) -> BTreeMap<String, ForeignKeyDef> {
  let mut fks = BTreeMap::new();
  for col in columns {
    let Some(refs) = &col.references else {
      continue;
    };
    let Some((ref_table, col_with_paren)) = refs.split_once('(') else {
      continue;
    };
    let ref_col = col_with_paren.trim_end_matches(')');
    let fk_name = column_fk_name(table_name, &col.name);
    fks.insert(
      fk_name.clone(),
      ForeignKeyDef {
        name: fk_name,
        columns: vec![col.name.clone()],
        references_table: ref_table.to_owned(),
        references_columns: vec![ref_col.to_owned()],
        on_delete: col.on_delete,
        on_update: col.on_update,
      },
    );
  }
  fks
}

pub(crate) fn extract_check_constraints(columns: &[ColumnDef]) -> BTreeMap<String, String> {
  let mut checks = BTreeMap::new();
  for col in columns {
    if let Some(check) = &col.check {
      checks.insert(col.name.clone(), check.clone());
    }
  }
  checks
}

pub(crate) fn strip_fk_from_column_snapshot(col: &mut ColumnDef) {
  col.references = None;
  col.on_delete = None;
  col.on_update = None;
}

pub(crate) fn strip_check_from_column_snapshot(col: &mut ColumnDef) {
  col.check = None;
}

/// Snapshot columns omit inline FK / CHECK when stored in `foreign_keys` / `check_constraints`.
pub(crate) fn columns_for_snapshot_table(columns: &[ColumnDef]) -> BTreeMap<String, ColumnDef> {
  columns
    .iter()
    .map(|c| {
      let mut c = c.clone();
      if c.references.is_some() {
        strip_fk_from_column_snapshot(&mut c);
      }
      if c.check.is_some() {
        strip_check_from_column_snapshot(&mut c);
      }
      (c.name.clone(), c)
    })
    .collect()
}

/// Moves every column-derived foreign key back onto its column and returns
/// the table-level ones, in name order.
pub(crate) fn merge_fk_into_columns(
  table_name: &str,
  table: &mut super::types::SnapshotTable,
) -> Vec<ForeignKeyDef> {
  let mut table_level = Vec::new();
  for fk in table.foreign_keys.values() {
    if !is_column_derived(table_name, fk, table.columns.keys()) {
      table_level.push(fk.clone());
      continue;
    }
    let (Some(col_name), Some(ref_col)) = (fk.columns.first(), fk.references_columns.first())
    else {
      continue;
    };
    let Some(col) = table.columns.get_mut(col_name) else {
      continue;
    };
    col.references = Some(format!("{}({ref_col})", fk.references_table));
    col.on_delete = fk.on_delete;
    col.on_update = fk.on_update;
  }
  table_level
}

pub(crate) fn merge_checks_into_columns(table: &mut super::types::SnapshotTable) {
  for (col_name, expr) in &table.check_constraints {
    let Some(col) = table.columns.get_mut(col_name) else {
      continue;
    };
    col.check = Some(expr.clone());
  }
}
