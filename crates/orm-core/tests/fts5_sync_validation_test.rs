//! Declarations the generator refuses, so a schema that cannot be
//! synchronized never writes a migration.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::fts5::{Fts5Sync, Fts5Table};
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableDef;

use schema::{memories, memory_fts, registry, snapshot_of, sync_refusal};

/// The refusal for a registry holding `memories` plus `table`, diffed from an
/// empty database.
fn refusal_for(table: TableDef) -> String {
  sync_refusal(&Snapshot::empty(), &registry(vec![memories(), table]))
}

#[test]
fn a_declaration_with_no_content_table_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: it sets no content table; a contentless index stores its own rows and needs no \
     triggers"
  );
}

#[test]
fn a_contentless_declaration_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: it sets no content table; a contentless index stores its own rows and needs no \
     triggers"
  );
}

#[test]
fn a_declaration_without_content_rowid_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("memories")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: it sets no content_rowid; the update trigger has to name that column to watch it"
  );
}

#[test]
fn a_declaration_with_no_columns_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: it declares no columns, so there is nothing to synchronize"
  );
}

#[test]
fn a_content_table_outside_the_schema_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("archives")
    .content_rowid("id")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: its content table \"archives\" is not in the schema"
  );
}

#[test]
fn a_virtual_content_table_is_refused() {
  let archive_fts = Fts5Table::new("archive_fts")
    .column("body", ColumnType::Text)
    .build();
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("archive_fts")
    .content_rowid("body")
    .sync_content()
    .build();
  assert_eq!(
    sync_refusal(&Snapshot::empty(), &registry(vec![archive_fts, table])),
    "memory_fts: its content table \"archive_fts\" is not an ordinary table"
  );
}

#[test]
fn a_content_table_missing_an_fts_column_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .column("headline", ColumnType::Text)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: its content table \"memories\" is missing column \"headline\""
  );
}

#[test]
fn a_content_table_missing_the_rowid_column_is_refused() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("memories")
    .content_rowid("memory_id")
    .sync_content()
    .build();
  assert_eq!(
    refusal_for(table),
    "memory_fts: its content table \"memories\" is missing content_rowid column \"memory_id\""
  );
}

#[test]
fn a_declaration_on_a_table_that_is_not_fts5_is_refused() {
  let mut table = memories();
  table.name = "notes".to_owned();
  table.fts5_sync = Some(Fts5Sync {
    content_table: "memories".to_owned(),
    content_rowid: "id".to_owned(),
    columns: vec!["body".to_owned()],
    indexed_columns: vec!["body".to_owned()],
  });
  assert_eq!(
    sync_refusal(&Snapshot::empty(), &registry(vec![memories(), table])),
    "notes: only an fts5 virtual table can synchronize a content table"
  );
}

#[test]
fn the_error_points_at_the_hand_written_alternative() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .content("memories")
    .sync_content()
    .build();
  let message = format!(
    "{}",
    toolu_orm_core::diff::diff(&Snapshot::empty(), &registry(vec![memories(), table]))
      .err()
      .unwrap_or(toolu_orm_core::error::DbCoreError::RowMapping(
        "none".to_owned()
      ))
  );
  assert!(
    message.contains("Drop sync_content() to keep writing the triggers by hand"),
    "error does not name the way out: {message}"
  );
}

#[test]
fn a_valid_declaration_is_not_refused() {
  let ops = toolu_orm_core::diff::diff(
    &snapshot_of(vec![memories()]),
    &registry(vec![memories(), memory_fts()]),
  );
  assert!(ops.is_ok(), "valid declaration was refused: {ops:?}");
}
