//! The trigger lifecycle for the index itself: created with a new declaration,
//! dropped when it is removed, and replaced when the source, the rowid, or the
//! indexed columns change.
//!
//! What happens when the tables around it change is in
//! `fts5_sync_rebuild_diff_test.rs`.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::snapshot::Snapshot;

use schema::{
  memories, memory_fts, memory_fts_named, memory_fts_unsynced, occurrences, offset_of, registry,
  snapshot_of, sqlite_sql,
};

const DROP_INSERT: &str = "DROP TRIGGER IF EXISTS \"toolu_fts5_memory_fts_insert\"";
const CREATE_INSERT: &str = "CREATE TRIGGER \"toolu_fts5_memory_fts_insert\"";
const REBUILD: &str = "INSERT INTO \"memory_fts\"(\"memory_fts\") VALUES('rebuild');";

#[test]
fn a_new_declaration_creates_the_triggers_and_rebuilds_without_a_drop() {
  let sql = sqlite_sql(
    &Snapshot::empty(),
    &registry(vec![memories(), memory_fts()]),
  );
  assert_eq!(occurrences(&sql, "CREATE TRIGGER"), 3, "{sql}");
  assert_eq!(occurrences(&sql, REBUILD), 1, "{sql}");
  assert!(
    !sql.contains("DROP TRIGGER"),
    "a first migration should not drop triggers that cannot exist: {sql}"
  );
  assert!(
    offset_of(&sql, "CREATE TABLE IF NOT EXISTS \"memories\"") < offset_of(&sql, CREATE_INSERT),
    "trigger created before its content table: {sql}"
  );
  assert!(
    offset_of(&sql, "CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\"")
      < offset_of(&sql, CREATE_INSERT),
    "trigger created before its index: {sql}"
  );
  assert!(
    offset_of(&sql, CREATE_INSERT) < offset_of(&sql, REBUILD),
    "rebuild ran before the triggers existed: {sql}"
  );
}

#[test]
fn adding_the_declaration_to_an_existing_index_replaces_nothing_but_the_triggers() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts_unsynced()]),
    &registry(vec![memories(), memory_fts()]),
  );
  assert_eq!(occurrences(&sql, "CREATE TRIGGER"), 3, "{sql}");
  assert_eq!(occurrences(&sql, REBUILD), 1, "{sql}");
  assert!(
    !sql.contains("DROP TABLE") && !sql.contains("CREATE VIRTUAL TABLE"),
    "the index was recreated for a trigger-only change: {sql}"
  );
}

#[test]
fn removing_the_declaration_drops_the_triggers_and_creates_none() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories(), memory_fts_unsynced()]),
  );
  assert_eq!(occurrences(&sql, "DROP TRIGGER IF EXISTS"), 3, "{sql}");
  assert!(!sql.contains("CREATE TRIGGER"), "{sql}");
  assert!(!sql.contains(REBUILD), "{sql}");
}

#[test]
fn dropping_the_index_drops_its_triggers_first() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories()]),
  );
  assert_eq!(occurrences(&sql, "DROP TRIGGER IF EXISTS"), 3, "{sql}");
  assert!(
    offset_of(&sql, DROP_INSERT) < offset_of(&sql, "DROP TABLE IF EXISTS \"memory_fts\""),
    "the table went before its triggers: {sql}"
  );
}

#[test]
fn an_unchanged_schema_emits_nothing() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories(), memory_fts()]),
  );
  assert_eq!(sql.trim(), "", "unchanged schema produced SQL: {sql}");
}

#[test]
fn a_tokenizer_change_recreates_the_table_then_its_triggers_then_the_index() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![
      memories(),
      memory_fts_named("memory_fts", "unicode61"),
    ]),
  );
  let order = [
    DROP_INSERT,
    "DROP TABLE IF EXISTS \"memory_fts\"",
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\"",
    CREATE_INSERT,
    REBUILD,
  ];
  for pair in order.windows(2) {
    let [before, after] = pair else {
      continue;
    };
    assert!(
      offset_of(&sql, before) < offset_of(&sql, after),
      "{before} did not come before {after}: {sql}"
    );
  }
  assert_eq!(occurrences(&sql, REBUILD), 1, "index rebuilt twice: {sql}");
}

#[test]
fn changing_the_content_rowid_replaces_the_triggers() {
  let changed = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .unindexed_column("note", ColumnType::Text)
    .tokenize("porter")
    .content("memories")
    .content_rowid("note")
    .sync_content()
    .build();
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories(), changed]),
  );
  assert_eq!(occurrences(&sql, "DROP TRIGGER IF EXISTS"), 3, "{sql}");
  assert!(
    sql.contains("AFTER UPDATE OF \"note\", \"body\" ON \"memories\""),
    "the new triggers still watch the old rowid: {sql}"
  );
}

#[test]
fn changing_the_indexed_columns_replaces_the_triggers() {
  let changed = Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .column("note", ColumnType::Text)
    .tokenize("porter")
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories(), changed]),
  );
  assert_eq!(occurrences(&sql, "DROP TRIGGER IF EXISTS"), 3, "{sql}");
  assert!(
    sql.contains("AFTER UPDATE OF \"id\", \"body\", \"note\" ON \"memories\""),
    "\"note\" did not become an indexed column: {sql}"
  );
}
