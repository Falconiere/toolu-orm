//! Enum type diff detection between snapshots.

use std::collections::BTreeSet;

use crate::snapshot::Snapshot;

use super::operation::Operation;

pub fn diff_enums(old: &Snapshot, new: &Snapshot) -> Vec<Operation> {
  let mut ops = Vec::new();

  for name in old.enums.keys() {
    if !new.enums.contains_key(name) {
      ops.push(Operation::DropEnum { name: name.clone() });
    }
  }

  for (name, new_enum) in &new.enums {
    let Some(old_enum) = old.enums.get(name) else {
      ops.push(Operation::CreateEnum {
        name: name.clone(),
        variants: new_enum.variants.clone(),
      });
      continue;
    };

    let old_v: BTreeSet<_> = old_enum.variants.iter().cloned().collect();
    let new_v: BTreeSet<_> = new_enum.variants.iter().cloned().collect();

    let added: Vec<String> = new_v.difference(&old_v).cloned().collect();
    let removed: Vec<String> = old_v.difference(&new_v).cloned().collect();

    if !added.is_empty() || !removed.is_empty() {
      ops.push(Operation::AlterEnum {
        name: name.clone(),
        added,
        removed,
      });
    }
  }

  ops
}
