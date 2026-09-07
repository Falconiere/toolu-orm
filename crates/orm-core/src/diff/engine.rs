//! Schema diff algorithm comparing two snapshots to produce migration operations.

use std::collections::{BTreeMap, BTreeSet};

use crate::column::ColumnDef;
use crate::index::IndexDef;
use crate::rename::{NoRenames, RenameResolver};
use crate::schema::SchemaRegistry;
use crate::snapshot::Snapshot;
use crate::table::TableDef;

use super::column::compute_column_changes;
use super::enums::diff_enums;
use super::fk::{diff_check_constraints_inner, diff_foreign_keys_inner};
use super::operation::Operation;

pub fn diff(old_snapshot: &Snapshot, new_schema: &SchemaRegistry) -> Vec<Operation> {
  diff_with_resolver(old_snapshot, new_schema, &NoRenames)
}

pub fn diff_with_resolver(
  old_snapshot: &Snapshot,
  new_schema: &SchemaRegistry,
  resolver: &impl RenameResolver,
) -> Vec<Operation> {
  let new_snap = Snapshot::from_registry(new_schema);
  let mut ops = diff_enums(old_snapshot, &new_snap);
  ops.extend(diff_tables(old_snapshot, &new_snap, new_schema, resolver));
  ops
}

fn diff_tables(
  old_snapshot: &Snapshot,
  new_snap: &Snapshot,
  new_schema: &SchemaRegistry,
  resolver: &impl RenameResolver,
) -> Vec<Operation> {
  let mut ops = Vec::new();
  let old_names: BTreeSet<String> = old_snapshot.tables.keys().cloned().collect();
  let new_names: BTreeSet<String> = new_snap.tables.keys().cloned().collect();

  let mut added: Vec<String> = new_names.difference(&old_names).cloned().collect();
  let mut removed: Vec<String> = old_names.difference(&new_names).cloned().collect();

  let renames = resolver.resolve_tables(&added, &removed);
  for (old_n, new_n) in &renames {
    added.retain(|x| x != new_n);
    removed.retain(|x| x != old_n);
    ops.push(Operation::RenameTable {
      old: old_n.clone(),
      new: new_n.clone(),
    });
  }

  for name in &removed {
    ops.push(Operation::DropTable { name: name.clone() });
  }

  let mut new_to_old: BTreeMap<String, String> = BTreeMap::new();
  for t in new_schema.tables() {
    new_to_old.insert(t.name.clone(), t.name.clone());
  }
  for (o, n) in &renames {
    new_to_old.insert(n.clone(), o.clone());
  }

  let pure_added: BTreeSet<String> = added.iter().cloned().collect();

  for table in new_schema.tables() {
    if pure_added.contains(&table.name) {
      ops.push(Operation::CreateTable {
        table: table.clone(),
      });
      for idx in &table.indexes {
        ops.push(Operation::CreateIndex {
          table: table.name.clone(),
          index: idx.clone(),
        });
      }
      continue;
    }

    let old_name = new_to_old
      .get(&table.name)
      .cloned()
      .unwrap_or_else(|| table.name.clone());
    let Some(old_st) = old_snapshot.tables.get(&old_name) else {
      continue;
    };
    let Some(new_st) = new_snap.tables.get(&table.name) else {
      continue;
    };

    diff_columns_for_table(
      &mut ops,
      &table.name,
      &old_st.columns,
      &new_st.columns,
      table,
      resolver,
    );
    diff_indexes_inner(&mut ops, &table.name, &old_st.indexes, &new_st.indexes);
    diff_foreign_keys_inner(
      &mut ops,
      &table.name,
      &old_st.foreign_keys,
      &new_st.foreign_keys,
    );
    diff_check_constraints_inner(
      &mut ops,
      &table.name,
      &old_st.check_constraints,
      &new_st.check_constraints,
    );
  }

  ops
}

fn diff_columns_for_table(
  ops: &mut Vec<Operation>,
  table_name: &str,
  old_cols: &BTreeMap<String, ColumnDef>,
  new_cols: &BTreeMap<String, ColumnDef>,
  new_table_def: &TableDef,
  resolver: &impl RenameResolver,
) {
  let added: Vec<String> = new_cols
    .keys()
    .filter(|k| !old_cols.contains_key(*k))
    .cloned()
    .collect();
  let removed: Vec<String> = old_cols
    .keys()
    .filter(|k| !new_cols.contains_key(*k))
    .cloned()
    .collect();

  let renames = resolver.resolve_columns(table_name, &added, &removed);
  let renamed_old: Vec<&str> = renames.iter().map(|(o, _)| o.as_str()).collect();
  let renamed_new: Vec<&str> = renames.iter().map(|(_, n)| n.as_str()).collect();

  for (old_name, new_name) in &renames {
    ops.push(Operation::RenameColumn {
      table: table_name.to_owned(),
      old: old_name.clone(),
      new: new_name.clone(),
    });
  }

  for name in &removed {
    if !renamed_old.contains(&name.as_str()) {
      ops.push(Operation::DropColumn {
        table: table_name.to_owned(),
        column: name.clone(),
      });
    }
  }

  for name in &added {
    if !renamed_new.contains(&name.as_str()) {
      if let Some(col) = new_cols.get(name) {
        ops.push(Operation::AddColumn {
          table: table_name.to_owned(),
          column: col.clone(),
        });
      }
    }
  }

  for name in new_cols.keys() {
    let Some(old_col) = old_cols.get(name) else {
      continue;
    };
    let new_col = &new_cols[name];
    let changes = compute_column_changes(name, old_col, new_col);
    if !changes.is_empty() {
      ops.push(Operation::AlterColumn {
        table: table_name.to_owned(),
        changes,
        table_def: new_table_def.clone(),
      });
    }
  }
}

fn diff_indexes_inner(
  ops: &mut Vec<Operation>,
  table_name: &str,
  old_indexes: &BTreeMap<String, IndexDef>,
  new_indexes: &BTreeMap<String, IndexDef>,
) {
  for (name, old_idx) in old_indexes {
    match new_indexes.get(name) {
      None => {
        ops.push(Operation::DropIndex { name: name.clone() });
      },
      Some(new_idx) if new_idx != old_idx => {
        ops.push(Operation::DropIndex { name: name.clone() });
        ops.push(Operation::CreateIndex {
          table: table_name.to_owned(),
          index: new_idx.clone(),
        });
      },
      _ => {},
    }
  }
  for (name, new_idx) in new_indexes {
    if !old_indexes.contains_key(name) {
      ops.push(Operation::CreateIndex {
        table: table_name.to_owned(),
        index: new_idx.clone(),
      });
    }
  }
}
