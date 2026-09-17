//! The trigger lifecycle when the tables around the index change: a content
//! table SQLite has to rebuild, a renamed index, and two indexes sharing one
//! content table.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::snapshot::Snapshot;

use schema::{
  assert_order, memories, memories_body_not_null, memory_fts, memory_fts_named,
  memory_fts_unsynced, occurrences, registry, snapshot_of, sqlite_sql, sqlite_sql_with_resolver,
  RenameOneTable,
};

const DROP_INSERT: &str = "DROP TRIGGER IF EXISTS \"toolu_fts5_memory_fts_insert\"";
const CREATE_INSERT: &str = "CREATE TRIGGER \"toolu_fts5_memory_fts_insert\"";
const REBUILD: &str = "INSERT INTO \"memory_fts\"(\"memory_fts\") VALUES('rebuild');";

#[test]
fn a_content_table_rebuild_puts_the_triggers_back_and_reindexes() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories_body_not_null(), memory_fts()]),
  );
  assert_eq!(occurrences(&sql, "DROP TRIGGER IF EXISTS"), 3, "{sql}");
  assert_eq!(occurrences(&sql, "CREATE TRIGGER"), 3, "{sql}");
  assert_order(&sql, DROP_INSERT, "CREATE TABLE \"_toolu_new_memories\"");
  assert_order(
    &sql,
    "ALTER TABLE \"_toolu_new_memories\" RENAME TO \"memories\"",
    CREATE_INSERT,
  );
  assert_order(&sql, CREATE_INSERT, REBUILD);
  assert!(
    !sql.contains("CREATE VIRTUAL TABLE"),
    "the index was recreated for a content-table change: {sql}"
  );
}

#[test]
fn a_content_table_rebuild_without_a_declaration_emits_no_triggers() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts_unsynced()]),
    &registry(vec![memories_body_not_null(), memory_fts_unsynced()]),
  );
  assert!(
    sql.contains("CREATE TABLE \"_toolu_new_memories\""),
    "{sql}"
  );
  assert!(!sql.contains("TRIGGER"), "{sql}");
}

#[test]
fn recreating_the_index_and_rebuilding_its_content_table_emits_one_pair() {
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![
      memories_body_not_null(),
      memory_fts_named("memory_fts", "unicode61"),
    ]),
  );
  assert_eq!(occurrences(&sql, DROP_INSERT), 1, "{sql}");
  assert_eq!(occurrences(&sql, CREATE_INSERT), 1, "{sql}");
  assert_eq!(occurrences(&sql, REBUILD), 1, "{sql}");
}

#[test]
fn renaming_the_index_drops_the_old_trigger_names_and_creates_the_new_ones() {
  let sql = sqlite_sql_with_resolver(
    &snapshot_of(vec![memories(), memory_fts()]),
    &registry(vec![memories(), memory_fts_named("note_fts", "porter")]),
    &RenameOneTable {
      old: "memory_fts",
      new: "note_fts",
    },
  );
  assert!(
    sql.contains(DROP_INSERT),
    "old trigger name not dropped: {sql}"
  );
  assert!(
    sql.contains("CREATE TRIGGER \"toolu_fts5_note_fts_insert\""),
    "new trigger name not created: {sql}"
  );
  assert_order(
    &sql,
    DROP_INSERT,
    "CREATE TRIGGER \"toolu_fts5_note_fts_insert\"",
  );
}

#[test]
fn two_indexes_over_one_content_table_get_their_own_triggers() {
  let substring = Fts5Table::new("memory_substring")
    .column("body", ColumnType::Text)
    .tokenize("trigram")
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = sqlite_sql(
    &Snapshot::empty(),
    &registry(vec![memories(), memory_fts(), substring]),
  );
  assert_eq!(occurrences(&sql, "CREATE TRIGGER"), 6, "{sql}");
  for name in [
    "toolu_fts5_memory_fts_insert",
    "toolu_fts5_memory_fts_delete",
    "toolu_fts5_memory_fts_update",
    "toolu_fts5_memory_substring_insert",
    "toolu_fts5_memory_substring_delete",
    "toolu_fts5_memory_substring_update",
  ] {
    assert_eq!(
      occurrences(&sql, name),
      1,
      "{name} is not created exactly once: {sql}"
    );
  }
}

#[test]
fn recreating_one_of_two_indexes_leaves_the_other_alone() {
  let substring = Fts5Table::new("memory_substring")
    .column("body", ColumnType::Text)
    .tokenize("trigram")
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = sqlite_sql(
    &snapshot_of(vec![memories(), memory_fts(), substring.clone()]),
    &registry(vec![
      memories(),
      memory_fts_named("memory_fts", "unicode61"),
      substring,
    ]),
  );
  assert_eq!(occurrences(&sql, "CREATE TRIGGER"), 3, "{sql}");
  assert!(
    !sql.contains("memory_substring"),
    "the untouched index was resynchronized: {sql}"
  );
}
