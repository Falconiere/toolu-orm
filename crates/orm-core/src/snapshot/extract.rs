//! Foreign key and constraint extraction from column definitions into snapshot format.

use std::collections::BTreeMap;

use crate::column::ColumnDef;

use super::types::ForeignKeyDef;

pub(crate) fn extract_foreign_keys(
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
    let fk_name = format!("fk_{table_name}_{}", col.name);
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

pub(crate) fn merge_fk_into_columns(table: &mut super::types::SnapshotTable) {
  for fk in table.foreign_keys.values() {
    for (i, col_name) in fk.columns.iter().enumerate() {
      let Some(col) = table.columns.get_mut(col_name) else {
        continue;
      };
      let ref_col = fk
        .references_columns
        .get(i)
        .map(String::as_str)
        .unwrap_or("id");
      col.references = Some(format!("{}({ref_col})", fk.references_table));
      col.on_delete = fk.on_delete;
      col.on_update = fk.on_update;
    }
  }
}

pub(crate) fn merge_checks_into_columns(table: &mut super::types::SnapshotTable) {
  for (col_name, expr) in &table.check_constraints {
    let Some(col) = table.columns.get_mut(col_name) else {
      continue;
    };
    col.check = Some(expr.clone());
  }
}
