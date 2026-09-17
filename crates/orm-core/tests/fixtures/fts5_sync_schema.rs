//! The synchronized FTS5 schema the sync-trigger suites evolve, and how they
//! read a generated migration back.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{diff, diff_with_resolver};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};

pub fn column(name: &str, column_type: ColumnType, primary_key: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type,
    primary_key,
    not_null: primary_key,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}

/// The content table: an integer rowid column FTS5 can address rows by, one
/// column the index searches, and one it only stores.
pub fn memories() -> TableDef {
  TableDef {
    name: "memories".to_owned(),
    columns: vec![
      column("id", ColumnType::Integer, true),
      column("body", ColumnType::Text, false),
      column("note", ColumnType::Text, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
  }
}

/// `memories` with `body` made NOT NULL — the change SQLite can only apply by
/// rebuilding the table, which is what destroys triggers attached to it.
pub fn memories_body_not_null() -> TableDef {
  let mut table = memories();
  for col in &mut table.columns {
    if col.name == "body" {
      col.not_null = true;
    }
  }
  table
}

/// The synchronized index over `memories`, named and tokenized by the caller.
pub fn memory_fts_named(name: &str, tokenize: &str) -> TableDef {
  Fts5Table::new(name)
    .column("body", ColumnType::Text)
    .unindexed_column("note", ColumnType::Text)
    .tokenize(tokenize)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build()
}

pub fn memory_fts() -> TableDef {
  memory_fts_named("memory_fts", "porter")
}

/// The same table without the opt-in: an external-content index whose triggers
/// stay the application's own.
pub fn memory_fts_unsynced() -> TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .unindexed_column("note", ColumnType::Text)
    .tokenize("porter")
    .content("memories")
    .content_rowid("id")
    .build()
}

pub fn registry(tables: Vec<TableDef>) -> SchemaRegistry {
  SchemaRegistry::from_tables(tables)
}

pub fn snapshot_of(tables: Vec<TableDef>) -> Snapshot {
  Snapshot::from_registry(&registry(tables))
}

/// The migration SQL for `old → new`, or a description of the refusal, so a
/// failing assertion names what happened instead of asserting on nothing.
pub fn sql_for(old: &Snapshot, new: &SchemaRegistry, dialect: Dialect) -> String {
  match diff(old, new) {
    Ok(ops) => generate_sql_for(&ops, dialect),
    Err(e) => format!("-- REFUSED: {e}"),
  }
}

pub fn sqlite_sql(old: &Snapshot, new: &SchemaRegistry) -> String {
  sql_for(old, new, Dialect::Sqlite)
}

/// The same, through a rename resolver.
pub fn sqlite_sql_with_resolver(
  old: &Snapshot,
  new: &SchemaRegistry,
  resolver: &impl RenameResolver,
) -> String {
  match diff_with_resolver(old, new, resolver) {
    Ok(ops) => generate_sql_for(&ops, Dialect::Sqlite),
    Err(e) => format!("-- REFUSED: {e}"),
  }
}

/// The `Fts5SyncInvalid` reason, or a description of whatever came back.
pub fn sync_refusal(old: &Snapshot, new: &SchemaRegistry) -> String {
  match diff(old, new) {
    Err(DbCoreError::Fts5SyncInvalid { table, reason }) => format!("{table}: {reason}"),
    Err(other) => format!("a different error: {other}"),
    Ok(ops) => format!("no refusal at all, but {ops:?}"),
  }
}

/// Byte offset of `needle`, or `usize::MAX` when it is absent — so an ordering
/// assertion fails with both offsets instead of on a missing statement.
pub fn offset_of(haystack: &str, needle: &str) -> usize {
  haystack.find(needle).unwrap_or(usize::MAX)
}

pub fn occurrences(haystack: &str, needle: &str) -> usize {
  haystack.matches(needle).count()
}

/// Renames one table — the single rename these suites need.
pub struct RenameOneTable {
  pub old: &'static str,
  pub new: &'static str,
}

impl RenameResolver for RenameOneTable {
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    if added.iter().any(|a| a == self.new) && removed.iter().any(|r| r == self.old) {
      return vec![(self.old.to_owned(), self.new.to_owned())];
    }
    vec![]
  }

  fn resolve_columns(
    &self,
    _table: &str,
    _added: &[String],
    _removed: &[String],
  ) -> Vec<(String, String)> {
    vec![]
  }
}
