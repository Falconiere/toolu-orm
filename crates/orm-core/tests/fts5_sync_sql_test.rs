//! The SQL the two synchronization operations render, in both dialects.

#[path = "fixtures/fts5_sync_schema.rs"]
pub mod schema;

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::fts5::{Fts5Sync, Fts5Table};
use toolu_orm_core::sql::generate_sql_for;

use schema::memory_fts;

/// The declaration the table records. A table that recorded none yields the
/// empty declaration, whose rendered SQL fails every assertion below by name.
fn sync_of(table: &toolu_orm_core::table::TableDef) -> Fts5Sync {
  table.fts5_sync.clone().unwrap_or_default()
}

fn create_sql(table: &toolu_orm_core::table::TableDef, dialect: Dialect) -> String {
  generate_sql_for(
    &[Operation::CreateFts5SyncTriggers {
      table: table.name.clone(),
      sync: sync_of(table),
    }],
    dialect,
  )
}

#[test]
fn the_insert_trigger_writes_every_column_addressed_by_the_content_rowid() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert!(
    sql.contains(
      "CREATE TRIGGER \"toolu_fts5_memory_fts_insert\" AFTER INSERT ON \"memories\" BEGIN\n  \
       INSERT INTO \"memory_fts\" (\"rowid\", \"body\", \"note\") VALUES (new.\"id\", \
       new.\"body\", new.\"note\");\nEND;"
    ),
    "insert trigger not in the documented shape: {sql}"
  );
}

#[test]
fn the_delete_trigger_uses_the_fts5_delete_command_with_the_old_values() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert!(
    sql.contains(
      "CREATE TRIGGER \"toolu_fts5_memory_fts_delete\" AFTER DELETE ON \"memories\" BEGIN\n  \
       INSERT INTO \"memory_fts\" (\"memory_fts\", \"rowid\", \"body\", \"note\") VALUES \
       ('delete', old.\"id\", old.\"body\", old.\"note\");\nEND;"
    ),
    "delete trigger not in the documented shape: {sql}"
  );
}

#[test]
fn the_update_trigger_deletes_before_reinserting() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert!(
    sql.contains(
      "CREATE TRIGGER \"toolu_fts5_memory_fts_update\" AFTER UPDATE OF \"id\", \"body\" ON \
       \"memories\" BEGIN\n  INSERT INTO \"memory_fts\" (\"memory_fts\", \"rowid\", \"body\", \
       \"note\") VALUES ('delete', old.\"id\", old.\"body\", old.\"note\");\n  INSERT INTO \
       \"memory_fts\" (\"rowid\", \"body\", \"note\") VALUES (new.\"id\", new.\"body\", \
       new.\"note\");\nEND;"
    ),
    "update trigger not in the documented shape: {sql}"
  );
}

#[test]
fn the_update_trigger_is_armed_only_for_the_rowid_and_indexed_columns() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert!(
    sql.contains("AFTER UPDATE OF \"id\", \"body\" ON"),
    "unindexed \"note\" reached the UPDATE OF list: {sql}"
  );
}

#[test]
fn an_all_unindexed_table_arms_the_update_trigger_on_the_rowid_alone() {
  let table = Fts5Table::new("memory_fts")
    .unindexed_column("note", ColumnType::Text)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = create_sql(&table, Dialect::Sqlite);
  assert!(
    sql.contains("AFTER UPDATE OF \"id\" ON \"memories\""),
    "UPDATE OF list is wrong: {sql}"
  );
}

#[test]
fn a_content_rowid_that_is_also_an_indexed_column_is_listed_once() {
  let table = Fts5Table::new("memory_fts")
    .column("id", ColumnType::Text)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = create_sql(&table, Dialect::Sqlite);
  assert!(
    sql.contains("AFTER UPDATE OF \"id\" ON \"memories\""),
    "rowid column repeated in the UPDATE OF list: {sql}"
  );
}

#[test]
fn creating_the_triggers_ends_with_the_rebuild_command() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert!(
    sql
      .trim_end()
      .ends_with("INSERT INTO \"memory_fts\"(\"memory_fts\") VALUES('rebuild');"),
    "rebuild is not the last statement: {sql}"
  );
  assert_eq!(sql.matches("VALUES('rebuild')").count(), 1);
}

#[test]
fn dropping_the_triggers_names_all_three_and_tolerates_their_absence() {
  let sql = generate_sql_for(
    &[Operation::DropFts5SyncTriggers {
      table: "memory_fts".to_owned(),
    }],
    Dialect::Sqlite,
  );
  assert_eq!(
    sql.trim(),
    "DROP TRIGGER IF EXISTS \"toolu_fts5_memory_fts_insert\";\n\
     DROP TRIGGER IF EXISTS \"toolu_fts5_memory_fts_delete\";\n\
     DROP TRIGGER IF EXISTS \"toolu_fts5_memory_fts_update\";"
  );
}

#[test]
fn neither_name_can_end_a_statement() {
  let table = Fts5Table::new("memory\"_fts")
    .column("bo\"dy", ColumnType::Text)
    .content("memo\"ries")
    .content_rowid("i\"d")
    .sync_content()
    .build();
  // Exact text, not a substring: every identifier is double-quoted with its
  // embedded quote doubled, so none of them can end the statement.
  let sql = create_sql(&table, Dialect::Sqlite);
  let insert = sql
    .split("--> statement-breakpoint")
    .next()
    .unwrap_or_default();
  assert_eq!(
    insert.trim(),
    "CREATE TRIGGER \"toolu_fts5_memory\"\"_fts_insert\" AFTER INSERT ON \"memo\"\"ries\" \
     BEGIN\n  INSERT INTO \"memory\"\"_fts\" (\"rowid\", \"bo\"\"dy\") VALUES (new.\"i\"\"d\", \
     new.\"bo\"\"dy\");\nEND;"
  );
  assert_eq!(
    sql.matches("VALUES('rebuild')").count(),
    1,
    "the rebuild command lost its quoting: {sql}"
  );
}

#[test]
fn postgres_reports_the_skip_instead_of_emitting_trigger_ddl() {
  let created = create_sql(&memory_fts(), Dialect::Postgres);
  assert_eq!(
    created.trim(),
    "-- FTS5 synchronization triggers for \"memory_fts\" are SQLite-only; skipped for postgres"
  );
  let dropped = generate_sql_for(
    &[Operation::DropFts5SyncTriggers {
      table: "memory_fts".to_owned(),
    }],
    Dialect::Postgres,
  );
  assert_eq!(
    dropped.trim(),
    "-- FTS5 synchronization triggers for \"memory_fts\" are SQLite-only; skipped for postgres"
  );
  for forbidden in ["CREATE TRIGGER", "DROP TRIGGER", "VALUES('rebuild')"] {
    assert!(
      !created.contains(forbidden) && !dropped.contains(forbidden),
      "postgres emitted {forbidden}"
    );
  }
}

#[test]
fn a_newline_in_the_name_stays_inside_the_postgres_comment() {
  let table = Fts5Table::new("memory_fts\nDROP TABLE memories;")
    .column("body", ColumnType::Text)
    .content("memories")
    .content_rowid("id")
    .sync_content()
    .build();
  let sql = create_sql(&table, Dialect::Postgres);
  assert_eq!(
    sql.trim().lines().count(),
    1,
    "comment broke onto a second line: {sql}"
  );
}

#[test]
fn recreating_a_synchronized_table_leaves_the_rebuild_to_the_trigger_operation() {
  let sql = generate_sql_for(
    &[Operation::RecreateFts5FromContent {
      table: memory_fts(),
    }],
    Dialect::Sqlite,
  );
  assert!(sql.contains("DROP TABLE IF EXISTS \"memory_fts\""), "{sql}");
  assert!(
    sql.contains("CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\""),
    "{sql}"
  );
  assert!(
    !sql.contains("VALUES('rebuild')"),
    "recreate rebuilt the index before its triggers existed: {sql}"
  );
}
