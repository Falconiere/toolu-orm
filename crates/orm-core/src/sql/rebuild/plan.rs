//! Deciding which tables SQLite has to rebuild, and what each rebuild absorbs.

use std::collections::{BTreeMap, BTreeSet};

use crate::diff::{ColumnChange, Operation};
use crate::table::TableDef;

/// One table's rebuild: the target definition, plus the columns that already
/// exist on the live table and can therefore be copied across. A column the
/// same diff adds is left out so it takes its declared default.
pub(crate) struct RebuildPlan {
  /// The table as the registry now declares it.
  pub table: TableDef,
  /// Columns to carry over from the old table, in target order.
  pub copy_columns: Vec<String>,
}

/// An ordered operation, or the rebuild that swallowed a run of them.
pub(crate) enum SqliteStep {
  /// An operation the rebuild planner left alone.
  Operation(Operation),
  /// One coordinated table rebuild.
  Rebuild(RebuildPlan),
}

/// True when SQLite has no in-place `ALTER` for these column changes and the
/// table has to be rebuilt.
fn needs_recreation_sqlite(changes: &[ColumnChange]) -> bool {
  changes.iter().any(|c| {
    matches!(
      c,
      ColumnChange::Type { .. }
        | ColumnChange::Default { .. }
        | ColumnChange::Nullable { .. }
        | ColumnChange::Unique { .. }
        | ColumnChange::PrimaryKey { .. }
        | ColumnChange::Autoincrement { .. }
        | ColumnChange::CompositePrimaryKey { .. }
    )
  })
}

/// Replaces every column operation on a rebuilt table with one [`RebuildPlan`]
/// at the position of the first of them, so a table is rebuilt once however
/// many of its columns changed.
///
/// Index operations pass through: `DropIndex` is ordered before the rebuild,
/// and `CreateIndex` after it is a no-op because the rebuild already re-creates
/// every index the target `TableDef` declares.
pub(crate) fn plan_sqlite_rebuilds(ordered: Vec<Operation>) -> Vec<SqliteStep> {
  let rebuilt = tables_to_rebuild(&ordered);
  if rebuilt.is_empty() {
    return ordered.into_iter().map(SqliteStep::Operation).collect();
  }
  let added = added_columns(&ordered, &rebuilt);
  let mut emitted: BTreeSet<String> = BTreeSet::new();
  let mut steps: Vec<SqliteStep> = Vec::new();

  for op in ordered {
    let absorbed = column_op_table(&op)
      .filter(|name| rebuilt.contains_key(*name))
      .map(ToOwned::to_owned);
    let Some(name) = absorbed else {
      steps.push(SqliteStep::Operation(op));
      continue;
    };
    if !emitted.insert(name.clone()) {
      continue;
    }
    if let Some(table) = rebuilt.get(&name) {
      steps.push(SqliteStep::Rebuild(RebuildPlan {
        copy_columns: copy_columns(table, added.get(&name)),
        table: table.clone(),
      }));
    }
  }
  steps
}

/// The table a column-level operation targets, `None` for anything else.
fn column_op_table(op: &Operation) -> Option<&str> {
  if let Operation::AlterColumn { table, .. } = op {
    return Some(table);
  }
  if let Operation::AddColumn { table, .. } = op {
    return Some(table);
  }
  if let Operation::DropColumn { table, .. } = op {
    return Some(table);
  }
  None
}

/// Target definitions of the tables an `AlterColumn` forces SQLite to rebuild.
/// Virtual tables are excluded: they are recreated by their own operation.
fn tables_to_rebuild(ops: &[Operation]) -> BTreeMap<String, TableDef> {
  let mut out: BTreeMap<String, TableDef> = BTreeMap::new();
  for op in ops {
    let Operation::AlterColumn {
      table,
      changes,
      table_def,
    } = op
    else {
      continue;
    };
    if needs_recreation_sqlite(changes) && table_def.kind.is_ordinary() {
      out
        .entry(table.clone())
        .or_insert_with(|| table_def.clone());
    }
  }
  out
}

/// Columns the same diff adds to a rebuilt table; they exist only in the
/// target definition, so the copy must not select them from the old table.
fn added_columns(
  ops: &[Operation],
  rebuilt: &BTreeMap<String, TableDef>,
) -> BTreeMap<String, BTreeSet<String>> {
  let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
  for op in ops {
    let Operation::AddColumn { table, column } = op else {
      continue;
    };
    if rebuilt.contains_key(table) {
      out
        .entry(table.clone())
        .or_default()
        .insert(column.name.clone());
    }
  }
  out
}

/// Target columns minus the ones this diff is adding.
fn copy_columns(table: &TableDef, added: Option<&BTreeSet<String>>) -> Vec<String> {
  table
    .columns
    .iter()
    .map(|c| c.name.clone())
    .filter(|name| !added.is_some_and(|names| names.contains(name)))
    .collect()
}
