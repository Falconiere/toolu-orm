//! Refusal rules for virtual tables.
//!
//! SQLite has no `ALTER TABLE` for a virtual table, and changing an FTS5
//! `tokenize` chain changes the table's identity. The only real transition is
//! drop, recreate and repopulate — and repopulation needs source data the ORM
//! does not model. So the diff creates, drops and renames virtual tables, and
//! refuses every in-place change with a message naming the table.

use crate::error::DbCoreError;
use crate::snapshot::SnapshotTable;
use crate::table::TableDef;

/// Checked before emitting `CreateTable` for a table the snapshot has not seen.
pub(crate) fn check_new_virtual_table(table: &TableDef) -> Result<(), DbCoreError> {
  if table.is_virtual() && !table.indexes.is_empty() {
    return Err(refuse(&table.name, NO_INDEXES));
  }
  Ok(())
}

/// Compares a table that exists on both sides.
///
/// Returns `Ok(true)` when the table is virtual and unchanged, so the caller
/// skips the ordinary column, index and constraint diffs; `Ok(false)` when
/// neither side is virtual; and an error for any in-place change.
pub(crate) fn check_virtual_pair(
  name: &str,
  old: &SnapshotTable,
  new: &SnapshotTable,
) -> Result<bool, DbCoreError> {
  match (old.kind.module(), new.kind.module()) {
    (None, None) => return Ok(false),
    (None, Some(module)) => {
      return Err(refuse(
        name,
        &format!("it became a virtual table using {module}"),
      ))
    },
    (Some(module), None) => {
      return Err(refuse(
        name,
        &format!("it is no longer a virtual table using {module}"),
      ))
    },
    (Some(old_module), Some(new_module)) if old_module != new_module => {
      return Err(refuse(
        name,
        &format!("its module changed from {old_module} to {new_module}"),
      ))
    },
    (Some(_), Some(_)) => {},
  }
  if old.column_order != new.column_order || old.columns != new.columns {
    return Err(refuse(name, "its columns changed"));
  }
  if old.kind.args() != new.kind.args() {
    return Err(refuse(name, "its module arguments changed"));
  }
  if !new.indexes.is_empty() {
    return Err(refuse(name, NO_INDEXES));
  }
  Ok(true)
}

const NO_INDEXES: &str = "virtual tables cannot declare indexes";

fn refuse(table: &str, reason: &str) -> DbCoreError {
  DbCoreError::VirtualTableChange {
    table: table.to_owned(),
    reason: reason.to_owned(),
  }
}
