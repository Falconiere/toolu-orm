//! DDL for the three FTS5 synchronization triggers, and the `rebuild` that
//! follows them.
//!
//! The shapes are SQLite's own, from the external-content section of the FTS5
//! documentation: a plain insert, the special `'delete'` command, and an update
//! that issues the delete before reinserting. The update trigger is armed only
//! for the indexed columns and the rowid column, so a write that touches
//! nothing the index covers costs nothing.

use crate::dialect::Dialect;
use crate::fts5::{sync_trigger_names, Fts5Sync};

/// Separator between the statements one operation renders.
const BREAKPOINT: &str = "\n\n--> statement-breakpoint\n\n";

/// The three `DROP TRIGGER IF EXISTS` statements as one chunk.
///
/// `IF EXISTS` because the same drop covers a table that never had them (the
/// declaration was only just added) and one whose triggers a previous
/// migration already removed.
pub(crate) fn drop_sync_triggers_sql(fts_table: &str, dialect: Dialect) -> String {
  if !matches!(dialect, Dialect::Sqlite) {
    return skipped_comment(fts_table, dialect);
  }
  sync_trigger_names(fts_table)
    .iter()
    .map(|name| format!("DROP TRIGGER IF EXISTS {};", quote_ident(name)))
    .collect::<Vec<_>>()
    .join("\n")
}

/// The three `CREATE TRIGGER` statements, then the `rebuild` that indexes the
/// rows already in the content table.
pub(crate) fn create_sync_triggers_sql(
  fts_table: &str,
  sync: &Fts5Sync,
  dialect: Dialect,
) -> String {
  if !matches!(dialect, Dialect::Sqlite) {
    return skipped_comment(fts_table, dialect);
  }
  let [insert, delete, update] = sync_trigger_names(fts_table);
  let fts = quote_ident(fts_table);
  let content = quote_ident(&sync.content_table);
  let (insert, delete, update) = (
    quote_ident(&insert),
    quote_ident(&delete),
    quote_ident(&update),
  );
  [
    format!(
      "CREATE TRIGGER {insert} AFTER INSERT ON {content} BEGIN\n  {};\nEND;",
      insert_statement(&fts, sync)
    ),
    format!(
      "CREATE TRIGGER {delete} AFTER DELETE ON {content} BEGIN\n  {};\nEND;",
      delete_statement(&fts, fts_table, sync)
    ),
    format!(
      "CREATE TRIGGER {update} AFTER UPDATE OF {} ON {content} BEGIN\n  {};\n  {};\nEND;",
      armed_columns(sync),
      delete_statement(&fts, fts_table, sync),
      insert_statement(&fts, sync)
    ),
    rebuild_sql(fts_table),
  ]
  .join(BREAKPOINT)
}

/// `INSERT INTO "<fts>"("<fts>") VALUES('rebuild');` — the command that indexes
/// whatever the content table already holds.
pub(crate) fn rebuild_sql(fts_table: &str) -> String {
  let fts = quote_ident(fts_table);
  format!("INSERT INTO {fts}({fts}) VALUES('rebuild');")
}

/// Writes every FTS column from `new`, addressed by the content rowid.
fn insert_statement(fts: &str, sync: &Fts5Sync) -> String {
  format!(
    "INSERT INTO {fts} ({}) VALUES ({})",
    column_list(sync, None),
    value_list(sync, "new")
  )
}

/// The `'delete'` command. FTS5 reverses the index entries from the values the
/// row was indexed under, so every one of them is read from `old`.
fn delete_statement(fts: &str, fts_table: &str, sync: &Fts5Sync) -> String {
  format!(
    "INSERT INTO {fts} ({}) VALUES ('delete', {})",
    column_list(sync, Some(fts_table)),
    value_list(sync, "old")
  )
}

/// `"<fts>", "rowid", "<col>"…` — the command column leads when the statement
/// is a command rather than a plain insert.
fn column_list(sync: &Fts5Sync, command: Option<&str>) -> String {
  let mut names: Vec<String> = command.into_iter().map(quote_ident).collect();
  names.push(quote_ident("rowid"));
  names.extend(sync.columns.iter().map(|c| quote_ident(c)));
  names.join(", ")
}

/// The matching `alias."<col>"` values. A command's own literal is written by
/// the caller, so this list always starts at the rowid.
fn value_list(sync: &Fts5Sync, alias: &str) -> String {
  let mut values = vec![format!("{alias}.{}", quote_ident(&sync.content_rowid))];
  values.extend(
    sync
      .columns
      .iter()
      .map(|c| format!("{alias}.{}", quote_ident(c))),
  );
  values.join(", ")
}

/// The `UPDATE OF` list: the rowid column, then the indexed columns, each once.
/// Never empty — the rowid column is always there — so the trigger is always
/// armed for the writes that can invalidate the index.
fn armed_columns(sync: &Fts5Sync) -> String {
  let mut names = vec![sync.content_rowid.clone()];
  for column in &sync.indexed_columns {
    if !names.contains(column) {
      names.push(column.clone());
    }
  }
  names
    .iter()
    .map(|c| quote_ident(c))
    .collect::<Vec<_>>()
    .join(", ")
}

/// Postgres has no FTS5, so it reports the skip the way a virtual table does
/// instead of emitting DDL it cannot run. Line breaks in the name are folded
/// away so the name cannot end the comment.
fn skipped_comment(fts_table: &str, dialect: Dialect) -> String {
  format!(
    "-- FTS5 synchronization triggers for {} are SQLite-only; skipped for {}",
    quote_ident(fts_table).replace(['\n', '\r'], " "),
    dialect.as_str()
  )
}

/// A double-quoted identifier; an embedded double quote doubles.
fn quote_ident(ident: &str) -> String {
  format!("\"{}\"", ident.replace('"', "\"\""))
}
