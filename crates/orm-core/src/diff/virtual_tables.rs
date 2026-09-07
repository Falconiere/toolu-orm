//! Refusal rules for virtual tables.
//!
//! SQLite has no `ALTER TABLE` for a virtual table, and changing an FTS5
//! `tokenize` chain changes the table's identity. The only real transition is
//! drop, recreate and repopulate — and repopulation needs source data the ORM
//! does not model. So the diff creates, drops and renames virtual tables, and
//! refuses every in-place change with a message naming the table.

use crate::error::DbCoreError;
use crate::snapshot::SnapshotTable;
use crate::table::{TableDef, TableKind};

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
  if old.kind.is_ordinary() && new.kind.is_ordinary() {
    return Ok(false);
  }
  if old.kind.module() != new.kind.module() {
    return Err(refuse(name, &kind_reason(&old.kind, &new.kind)));
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

/// Only called when the two modules differ, so every arm names one.
fn kind_reason(old: &TableKind, new: &TableKind) -> String {
  match (old.module(), new.module()) {
    (Some(old_module), Some(new_module)) => {
      format!("its module changed from {old_module} to {new_module}")
    },
    (None, Some(new_module)) => format!("it became a virtual table using {new_module}"),
    (Some(old_module), None) => format!("it is no longer a virtual table using {old_module}"),
    (None, None) => "its table kind changed".to_owned(),
  }
}

fn refuse(table: &str, reason: &str) -> DbCoreError {
  DbCoreError::VirtualTableChange {
    table: table.to_owned(),
    reason: reason.to_owned(),
  }
}
