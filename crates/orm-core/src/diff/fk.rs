//! Foreign key and check constraint diff detection between snapshots.

use std::collections::BTreeMap;

use crate::snapshot::{ForeignKeyDef, Snapshot};

use super::operation::Operation;

pub fn diff_foreign_keys(table_name: &str, old: &Snapshot, new: &Snapshot) -> Vec<Operation> {
  let mut ops = Vec::new();
  let old_t = old.tables.get(table_name);
  let new_t = new.tables.get(table_name);
  let empty = BTreeMap::new();
  let old_fks = old_t.map_or(&empty, |t| &t.foreign_keys);
  let empty2 = BTreeMap::new();
  let new_fks = new_t.map_or(&empty2, |t| &t.foreign_keys);
  diff_foreign_keys_inner(&mut ops, table_name, old_fks, new_fks);
  ops
}

pub(crate) fn diff_foreign_keys_inner(
  ops: &mut Vec<Operation>,
  table_name: &str,
  old_fks: &BTreeMap<String, ForeignKeyDef>,
  new_fks: &BTreeMap<String, ForeignKeyDef>,
) {
  for (name, old_fk) in old_fks {
    match new_fks.get(name) {
      None => {
        ops.push(Operation::DropForeignKey {
          table: table_name.to_owned(),
          name: name.clone(),
        });
      },
      Some(new_fk) if new_fk != old_fk => {
        ops.push(Operation::DropForeignKey {
          table: table_name.to_owned(),
          name: name.clone(),
        });
        ops.push(Operation::AddForeignKey {
          table: table_name.to_owned(),
          fk: new_fk.clone(),
        });
      },
      _ => {},
    }
  }

  for (name, new_fk) in new_fks {
    if !old_fks.contains_key(name) {
      ops.push(Operation::AddForeignKey {
        table: table_name.to_owned(),
        fk: new_fk.clone(),
      });
    }
  }
}

pub(crate) fn diff_check_constraints_inner(
  ops: &mut Vec<Operation>,
  table_name: &str,
  old_checks: &BTreeMap<String, String>,
  new_checks: &BTreeMap<String, String>,
) {
  for (name, old_expr) in old_checks {
    match new_checks.get(name) {
      None => {
        ops.push(Operation::DropCheckConstraint {
          table: table_name.to_owned(),
          name: name.clone(),
        });
      },
      Some(new_expr) if new_expr != old_expr => {
        ops.push(Operation::DropCheckConstraint {
          table: table_name.to_owned(),
          name: name.clone(),
        });
        ops.push(Operation::AddCheckConstraint {
          table: table_name.to_owned(),
          name: name.clone(),
          expr: new_expr.clone(),
        });
      },
      _ => {},
    }
  }

  for (name, new_expr) in new_checks {
    if !old_checks.contains_key(name) {
      ops.push(Operation::AddCheckConstraint {
        table: table_name.to_owned(),
        name: name.clone(),
        expr: new_expr.clone(),
      });
    }
  }
}
