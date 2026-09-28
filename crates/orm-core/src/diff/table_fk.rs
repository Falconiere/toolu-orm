//! Table-level foreign-key changes on a table both snapshots contain.

use crate::snapshot::{table_level_foreign_keys, SnapshotTable};
use crate::table::TableDef;

use super::operation::{ColumnChange, Operation};

/// An `AlterColumn` carrying [`ColumnChange::TableForeignKeys`] when the
/// table-level foreign keys differ. Each side is classified under its own
/// table name, so renaming a table does not turn its column-level keys into
/// table-level ones.
pub(crate) fn table_foreign_key_change(
  old_name: &str,
  old: &SnapshotTable,
  new: &SnapshotTable,
  table: &TableDef,
) -> Option<Operation> {
  let old_fks = table_level_foreign_keys(old_name, old);
  let new_fks = table_level_foreign_keys(&table.name, new);
  if old_fks == new_fks {
    return None;
  }
  Some(Operation::AlterColumn {
    table: table.name.clone(),
    changes: vec![ColumnChange::TableForeignKeys {
      old: old_fks.into_values().collect(),
      new: new_fks.into_values().collect(),
    }],
    table_def: table.clone(),
  })
}
