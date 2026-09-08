//! The FTS5 table the virtual-table diff tests evolve, and the two ways they
//! read a diff back: the refusal message, or a rename resolver.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::diff::diff;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::rename::RenameResolver;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableDef;

pub const NO_INDEXES_REASON: &str = "virtual tables cannot declare indexes";

pub fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .tokenize("porter")
    .build()
}

pub fn registry(tables: Vec<TableDef>) -> SchemaRegistry {
  SchemaRegistry::from_tables(tables)
}

pub fn snapshot_of(tables: Vec<TableDef>) -> Snapshot {
  Snapshot::from_registry(&registry(tables))
}

/// The refusal message, or a description of whatever came back instead — the
/// caller compares it to the expected refusal, so either way the assertion
/// names what happened.
pub fn refusal(old: &Snapshot, new: &SchemaRegistry) -> String {
  match diff(old, new) {
    Err(DbCoreError::VirtualTableChange { table, reason }) => format!("{table}: {reason}"),
    Err(other) => format!("a different error: {other}"),
    Ok(ops) => format!("no refusal at all, but {ops:?}"),
  }
}

/// Renames `memory_fts` to `note_fts`, the one rename these tests need.
pub struct RenameMemoryFts;

impl RenameResolver for RenameMemoryFts {
  fn resolve_tables(&self, added: &[String], removed: &[String]) -> Vec<(String, String)> {
    if added.iter().any(|a| a == "note_fts") && removed.iter().any(|r| r == "memory_fts") {
      return vec![("memory_fts".to_owned(), "note_fts".to_owned())];
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
