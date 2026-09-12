//! Index-level diff between two snapshot index maps.

use std::collections::BTreeMap;

use crate::index::IndexDef;

use super::operation::Operation;

pub(crate) fn diff_indexes_inner(
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
