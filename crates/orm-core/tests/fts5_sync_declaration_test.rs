//! What `sync_content()` records on the table, and what the snapshot does
//! with it.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::fts5::{sync_trigger_names, Fts5Sync, Fts5Table};
use toolu_orm_core::snapshot::Snapshot;

use schema::{memories, memory_fts, memory_fts_sync, memory_fts_unsynced, registry, snapshot_of};

#[test]
fn sync_content_records_the_content_table_rowid_and_columns() {
  // The whole declaration against a literal, so neither a missing spec nor a
  // field added later can slip past.
  assert_eq!(memory_fts().fts5_sync, Some(memory_fts_sync()));
}

#[test]
fn without_the_opt_in_nothing_is_recorded() {
  assert_eq!(memory_fts_unsynced().fts5_sync, None);
  assert_eq!(memories().fts5_sync, None);
}

#[test]
fn an_all_unindexed_table_records_no_indexed_columns() {
  let table = Fts5Table::new("memory_fts")
    .unindexed_column("note", ColumnType::Text)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  assert_eq!(
    table.fts5_sync,
    Some(Fts5Sync {
      content_table: "memories".to_owned(),
      content_rowid: "id".to_owned(),
      columns: vec!["note".to_owned()],
      indexed_columns: vec![],
    })
  );
}

#[test]
fn a_missing_content_option_is_recorded_as_empty_rather_than_dropped() {
  let table = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .sync_content()
    .build();
  assert_eq!(
    table.fts5_sync,
    Some(Fts5Sync {
      content_table: String::new(),
      content_rowid: String::new(),
      columns: vec!["body".to_owned()],
      indexed_columns: vec!["body".to_owned()],
    })
  );
}

#[test]
fn trigger_names_key_on_the_fts_table_alone() {
  assert_eq!(
    sync_trigger_names("memory_fts").all(),
    [
      "toolu_fts5_memory_fts_insert",
      "toolu_fts5_memory_fts_delete",
      "toolu_fts5_memory_fts_update",
    ]
  );
  assert_ne!(
    sync_trigger_names("memory_fts"),
    sync_trigger_names("memory_substring")
  );
}

#[test]
fn the_snapshot_round_trip_preserves_the_declaration() {
  let json =
    serde_json::to_string(&snapshot_of(vec![memories(), memory_fts()])).unwrap_or_default();
  let parsed: Snapshot = serde_json::from_str(&json).unwrap_or_else(|_| Snapshot::empty());
  let restored = parsed.to_registry();
  assert_eq!(
    restored
      .find_table("memory_fts")
      .and_then(|t| t.fts5_sync.clone()),
    Some(memory_fts_sync())
  );
  assert_eq!(
    restored
      .find_table("memories")
      .and_then(|t| t.fts5_sync.clone()),
    None
  );
}

#[test]
fn a_table_without_the_declaration_writes_no_fts5_sync_key() {
  let json = serde_json::to_string(&snapshot_of(vec![memories(), memory_fts_unsynced()]))
    .unwrap_or_default();
  assert!(
    !json.contains("fts5_sync"),
    "unsynchronized schema wrote the key: {json}"
  );
}

#[test]
fn the_declaration_reaches_the_json_under_its_own_key() {
  let json =
    serde_json::to_string(&snapshot_of(vec![memories(), memory_fts()])).unwrap_or_default();
  assert!(
    json.contains(
      r#""fts5_sync":{"content_table":"memories","content_rowid":"id","columns":["body","note"],"indexed_columns":["body"]}"#
    ),
    "declaration missing or reshaped: {json}"
  );
}

#[test]
fn a_snapshot_written_before_this_existed_loads_as_unsynchronized() {
  let legacy = include_str!("fixtures/legacy_snapshot.json");
  let parsed: Snapshot = serde_json::from_str(legacy).unwrap_or_else(|_| Snapshot::empty());
  assert!(!parsed.tables.is_empty(), "legacy fixture parsed as empty");
  for (name, table) in &parsed.tables {
    assert_eq!(table.fts5_sync, None, "{name} came back synchronized");
  }
}

#[test]
fn from_registry_carries_the_declaration_into_the_snapshot_table() {
  let snapshot = Snapshot::from_registry(&registry(vec![memories(), memory_fts()]));
  assert_eq!(
    snapshot
      .tables
      .get("memory_fts")
      .and_then(|t| t.fts5_sync.clone()),
    Some(memory_fts_sync())
  );
}
