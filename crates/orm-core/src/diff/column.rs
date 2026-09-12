//! Column-level diff detection between old and new column definitions.

use crate::column::ColumnDef;

use super::operation::ColumnChange;

pub(crate) fn compute_column_changes(
  name: &str,
  old: &ColumnDef,
  new: &ColumnDef,
) -> Vec<ColumnChange> {
  let mut changes = Vec::new();

  if old.column_type != new.column_type {
    changes.push(ColumnChange::Type {
      column: name.to_owned(),
      old: old.column_type.clone(),
      new: new.column_type.clone(),
    });
  }
  if old.default != new.default {
    changes.push(ColumnChange::Default {
      column: name.to_owned(),
      old: old.default.clone(),
      new: new.default.clone(),
    });
  }
  if old.not_null != new.not_null {
    changes.push(ColumnChange::Nullable {
      column: name.to_owned(),
      old: !old.not_null,
      new: !new.not_null,
    });
  }
  if old.unique != new.unique {
    changes.push(ColumnChange::Unique {
      column: name.to_owned(),
      old: old.unique,
      new: new.unique,
    });
  }
  if old.primary_key != new.primary_key {
    changes.push(ColumnChange::PrimaryKey {
      column: name.to_owned(),
      old: old.primary_key,
      new: new.primary_key,
    });
  }
  if old.autoincrement != new.autoincrement {
    changes.push(ColumnChange::Autoincrement {
      column: name.to_owned(),
      old: old.autoincrement,
      new: new.autoincrement,
    });
  }

  changes
}
