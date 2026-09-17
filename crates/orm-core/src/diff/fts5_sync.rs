//! Validating FTS5 synchronization declarations, and planning the trigger
//! operations one diff needs.
//!
//! The rules overlap — a diff can recreate an FTS table *and* rebuild its
//! content table — so planning resolves per FTS table rather than per rule:
//! each contributes at most one [`Operation::DropFts5SyncTriggers`] and at most
//! one [`Operation::CreateFts5SyncTriggers`]. Ordering does the rest;
//! [`crate::ordering`] puts the drop in with the other drops and the create in
//! with the other creates.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::DbCoreError;
use crate::fts5::{Fts5Sync, FTS5_MODULE};
use crate::schema::SchemaRegistry;
use crate::snapshot::Snapshot;
use crate::table::TableDef;

use super::operation::Operation;
use super::virtual_tables::content_table_reason;

/// Rejects every declaration the generator cannot turn into triggers, before
/// any operation is produced — so a schema that cannot be synchronized writes
/// no migration at all.
///
/// # Errors
///
/// [`DbCoreError::Fts5SyncInvalid`] naming the table and the reason.
pub(crate) fn validate(schema: &SchemaRegistry) -> Result<(), DbCoreError> {
  for table in schema.tables() {
    let Some(sync) = &table.fts5_sync else {
      continue;
    };
    let Some(reason) = invalid_reason(table, sync, schema) else {
      continue;
    };
    return Err(DbCoreError::Fts5SyncInvalid {
      table: table.name.clone(),
      reason,
    });
  }
  Ok(())
}

/// Why this declaration cannot be synchronized, or `None` when it can.
fn invalid_reason(table: &TableDef, sync: &Fts5Sync, schema: &SchemaRegistry) -> Option<String> {
  if table.kind.module() != Some(FTS5_MODULE) {
    return Some("only an fts5 virtual table can synchronize a content table".to_owned());
  }
  if sync.columns.is_empty() {
    return Some("it declares no columns, so there is nothing to synchronize".to_owned());
  }
  if sync.content_table.is_empty() {
    return Some(
      "it sets no content table; a contentless index stores its own rows and needs no triggers"
        .to_owned(),
    );
  }
  if sync.content_rowid.is_empty() {
    return Some(
      "it sets no content_rowid; the update trigger has to name that column to watch it".to_owned(),
    );
  }
  content_table_reason(
    schema,
    &sync.content_table,
    &sync.columns,
    Some(&sync.content_rowid),
  )
}

/// The trigger operations this diff needs, given the operations it has already
/// produced.
pub(crate) fn trigger_operations(
  old_snapshot: &Snapshot,
  schema: &SchemaRegistry,
  renames: &[(String, String)],
  ops: &[Operation],
  added: &BTreeSet<String>,
) -> Vec<Operation> {
  let old_specs = old_specs(old_snapshot);
  let new_specs = new_specs(schema);
  let recreated = recreated_fts_tables(ops);
  let rebuilt = rebuilt_tables(ops);

  let mut drops: Vec<String> = Vec::new();
  let mut creates: Vec<Operation> = Vec::new();

  for (name, sync) in &new_specs {
    let old_name = previous_name(renames, name);
    let old_spec = old_specs.get(old_name);
    let unchanged = old_spec == Some(sync)
      && old_name == *name
      && !recreated.contains(name)
      && !rebuilt.contains(sync.content_table.as_str());
    if unchanged {
      continue;
    }
    // A table this diff creates has no triggers yet, so the drop would only be
    // noise in the migration that first declares it.
    if old_spec.is_some() && !added.contains(*name) {
      drops.push(old_name.to_owned());
    }
    creates.push(Operation::CreateFts5SyncTriggers {
      table: (*name).to_owned(),
      sync: (*sync).clone(),
    });
  }

  for old_name in old_specs.keys() {
    if new_specs.contains_key(current_name(renames, old_name)) {
      continue;
    }
    drops.push((*old_name).to_owned());
  }

  drops
    .into_iter()
    .map(|table| Operation::DropFts5SyncTriggers { table })
    .chain(creates)
    .collect()
}

/// The declarations the old snapshot recorded, by the name they had then.
fn old_specs(snapshot: &Snapshot) -> BTreeMap<&str, &Fts5Sync> {
  snapshot
    .tables
    .iter()
    .filter_map(|(name, table)| Some((name.as_str(), table.fts5_sync.as_ref()?)))
    .collect()
}

/// The declarations the registry now holds.
fn new_specs(schema: &SchemaRegistry) -> BTreeMap<&str, &Fts5Sync> {
  schema
    .tables()
    .iter()
    .filter_map(|table| Some((table.name.as_str(), table.fts5_sync.as_ref()?)))
    .collect()
}

/// What `name` was called before this diff, which is also the name its triggers
/// still carry: `ALTER TABLE … RENAME` renames the table, never its triggers.
fn previous_name<'a>(renames: &'a [(String, String)], name: &'a str) -> &'a str {
  renames
    .iter()
    .find(|(_, new)| new == name)
    .map_or(name, |(old, _)| old.as_str())
}

/// What `name` is called after this diff.
fn current_name<'a>(renames: &'a [(String, String)], name: &'a str) -> &'a str {
  renames
    .iter()
    .find(|(old, _)| old == name)
    .map_or(name, |(_, new)| new.as_str())
}

/// FTS tables this diff drops and recreates; their triggers go with them.
fn recreated_fts_tables(ops: &[Operation]) -> BTreeSet<&str> {
  let mut out = BTreeSet::new();
  for op in ops {
    let Operation::RecreateFts5FromContent { table } = op else {
      continue;
    };
    out.insert(table.name.as_str());
  }
  out
}

/// Tables an `AlterColumn` touches. On SQLite that is exactly the set the
/// generator rebuilds, and a rebuild's `DROP TABLE` takes every trigger
/// attached to the table with it.
fn rebuilt_tables(ops: &[Operation]) -> BTreeSet<&str> {
  let mut out = BTreeSet::new();
  for op in ops {
    let Operation::AlterColumn { table, .. } = op else {
      continue;
    };
    out.insert(table.as_str());
  }
  out
}
