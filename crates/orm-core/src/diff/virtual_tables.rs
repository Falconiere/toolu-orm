//! Virtual-table diff: create/drop/rename stay ordinary; in-place changes
//! either rebuild from FTS5 external content or refuse.
//!
//! SQLite has no `ALTER TABLE` for a virtual table. When the new FTS5
//! definition names a non-empty ordinary `content` table that covers every
//! FTS column (and `content_rowid` when set), the diff emits
//! [`Operation::RecreateFts5FromContent`]. Everything else that would mutate
//! a virtual table in place is still refused — including rename+change in
//! the same generate — because repopulation needs source data the schema
//! does not safely provide.

use crate::error::DbCoreError;
use crate::fts5::FTS5_MODULE;
use crate::schema::SchemaRegistry;
use crate::snapshot::SnapshotTable;
use crate::table::TableDef;

use std::collections::BTreeSet;

use super::operation::Operation;

/// Result of comparing a table present on both sides of the diff.
pub(crate) enum VirtualPairCheck {
  /// Neither side is virtual; run ordinary column / index / FK diffs.
  Ordinary,
  /// Virtual and unchanged; skip ordinary diffs.
  Unchanged,
  /// Emit this op instead of ordinary diffs or a refuse error.
  Recreate(Operation),
}

/// Checked before emitting `CreateTable` for a table the snapshot has not seen.
pub(crate) fn check_new_virtual_table(table: &TableDef) -> Result<(), DbCoreError> {
  if table.is_virtual() && !table.indexes.is_empty() {
    return Err(refuse(&table.name, NO_INDEXES));
  }
  Ok(())
}

/// Compares a table that exists on both sides.
///
/// `renamed` is true when this generate also renamed the table; a rename
/// combined with a shape change always refuses (never auto-rebuilds).
pub(crate) fn check_virtual_pair(
  name: &str,
  old: &SnapshotTable,
  new: &SnapshotTable,
  new_table: &TableDef,
  schema: &SchemaRegistry,
  renamed: bool,
) -> Result<VirtualPairCheck, DbCoreError> {
  match (old.kind.module(), new.kind.module()) {
    (None, None) => return Ok(VirtualPairCheck::Ordinary),
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

  let columns_changed = old.column_order != new.column_order || old.columns != new.columns;
  let args_changed = old.kind.args() != new.kind.args();
  if !columns_changed && !args_changed {
    if !new.indexes.is_empty() {
      return Err(refuse(name, NO_INDEXES));
    }
    return Ok(VirtualPairCheck::Unchanged);
  }

  if !new.indexes.is_empty() {
    return Err(refuse(name, NO_INDEXES));
  }

  let change_reason = if columns_changed {
    "its columns changed"
  } else {
    "its module arguments changed"
  };

  if renamed {
    return Err(refuse(name, change_reason));
  }

  if new.kind.module() == Some(FTS5_MODULE) {
    if let Some(op) = try_recreate_from_content(name, new_table, schema)? {
      return Ok(VirtualPairCheck::Recreate(op));
    }
  }

  Err(refuse(name, change_reason))
}

fn try_recreate_from_content(
  name: &str,
  new_table: &TableDef,
  schema: &SchemaRegistry,
) -> Result<Option<Operation>, DbCoreError> {
  let Some(content_name) = option_value(new_table.kind.args(), "content") else {
    return Ok(None);
  };
  if content_name.is_empty() {
    return Ok(None);
  }

  let Some(content) = schema.tables().iter().find(|t| t.name == content_name) else {
    return Err(refuse(
      name,
      &format!("its content table \"{content_name}\" is not in the schema"),
    ));
  };
  if content.is_virtual() {
    return Err(refuse(
      name,
      &format!("its content table \"{content_name}\" is not an ordinary table"),
    ));
  }

  let content_cols: BTreeSet<&str> = content.columns.iter().map(|c| c.name.as_str()).collect();
  for column in &new_table.columns {
    if !content_cols.contains(column.name.as_str()) {
      return Err(refuse(
        name,
        &format!(
          "its content table \"{content_name}\" is missing column \"{}\"",
          column.name
        ),
      ));
    }
  }

  if let Some(rowid) = option_value(new_table.kind.args(), "content_rowid") {
    if !content_cols.contains(rowid.as_str()) {
      return Err(refuse(
        name,
        &format!(
          "its content table \"{content_name}\" is missing content_rowid column \"{rowid}\""
        ),
      ));
    }
  }

  Ok(Some(Operation::RecreateFts5FromContent {
    table: new_table.clone(),
  }))
}

/// Reads `key = 'value'` from FTS5 module args (`Fts5Options::render` form).
fn option_value(args: &[String], key: &str) -> Option<String> {
  let prefix = format!("{key} = '");
  args.iter().find_map(|arg| {
    let rest = arg.strip_prefix(&prefix)?;
    let raw = rest.strip_suffix('\'')?;
    Some(raw.replace("''", "'"))
  })
}

const NO_INDEXES: &str = "virtual tables cannot declare indexes";

fn refuse(table: &str, reason: &str) -> DbCoreError {
  DbCoreError::VirtualTableChange {
    table: table.to_owned(),
    reason: reason.to_owned(),
  }
}
