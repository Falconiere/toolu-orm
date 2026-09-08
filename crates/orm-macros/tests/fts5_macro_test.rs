//! `#[fts5_table]` expansion: the virtual `TableDef`, the `UNINDEXED` flag,
//! the typed column module, and the builder factories.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::table::TableSchema;
use toolu_orm_core::value::Value;
use toolu_orm_macros::fts5_table;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[fts5_table(name = "memory_fts", tokenize = "porter unicode61 remove_diacritics 2")]
pub struct MemoryFts {
  #[column(unindexed)]
  pub memory_id: Text,
  pub body: Text,
  pub tags: Text,
}

#[fts5_table(
  name = "code_fts",
  tokenize = "identifier",
  prefix = "2 3",
  content = "symbols",
  content_rowid = "id",
  columnsize = 0,
  detail = "none"
)]
pub struct CodeFts {
  #[column(unindexed)]
  pub symbol_id: Text,
  pub symbol: Text,
  pub snippet: Text,
}

#[test]
fn table_def_is_a_virtual_fts5_table() {
  let def = MemoryFts::table_def();
  assert_eq!(def.name, "memory_fts");
  assert!(def.is_virtual());
  assert_eq!(def.kind.module(), Some("fts5"));
  assert_eq!(
    def.kind.args(),
    [
      "\"memory_id\" UNINDEXED",
      "\"body\"",
      "\"tags\"",
      "tokenize = 'porter unicode61 remove_diacritics 2'",
    ]
  );
  assert!(def.indexes.is_empty());
  assert!(!def.strict);
}

#[test]
fn only_the_marked_column_is_unindexed() -> TestResult {
  let def = MemoryFts::table_def();
  let memory_id = def.find_column("memory_id").ok_or("missing memory_id")?;
  let body = def.find_column("body").ok_or("missing body")?;
  assert!(memory_id.unindexed);
  assert!(!body.unindexed);
  Ok(())
}

#[test]
fn every_option_reaches_the_module_arguments() {
  let def = CodeFts::table_def();
  assert_eq!(
    def.kind.args(),
    [
      "\"symbol_id\" UNINDEXED",
      "\"symbol\"",
      "\"snippet\"",
      "prefix = '2 3'",
      "tokenize = 'identifier'",
      "content = 'symbols'",
      "content_rowid = 'id'",
      "columnsize = 0",
      "detail = 'none'",
    ]
  );
}

#[test]
fn the_column_module_is_generated() {
  assert_eq!(memory_fts::TABLE, "memory_fts");
  assert_eq!(memory_fts::ALL_COLUMNS, ["memory_id", "body", "tags"]);
  assert_eq!(memory_fts::body.name, "body");
  assert_eq!(memory_fts::body.qualified(), "\"memory_fts\".\"body\"");
}

/// The same four factories `#[table]` generates, each addressing the virtual
/// table by name — an FTS5 table is read and written like any other. The
/// dialect is pinned because `to_sql` follows the active driver feature and
/// this suite runs on the default and postgres lanes both.
#[test]
fn the_builder_factories_are_generated() {
  assert_eq!(
    MemoryFts::select()
      .columns_raw(memory_fts::ALL_COLUMNS)
      .to_sql_for(Dialect::Sqlite)
      .0,
    "SELECT \"memory_id\", \"body\", \"tags\" FROM \"memory_fts\""
  );
  assert_eq!(
    MemoryFts::insert()
      .set(&memory_fts::body, Value::Text("hello".to_owned()))
      .to_sql_for(Dialect::Sqlite)
      .0,
    "INSERT INTO \"memory_fts\" (\"body\") VALUES (?1)"
  );
  assert_eq!(
    MemoryFts::update()
      .set(&memory_fts::body, Value::Text("hello".to_owned()))
      .to_sql_for(Dialect::Sqlite)
      .0,
    "UPDATE \"memory_fts\" SET \"body\" = ?1"
  );
  assert_eq!(
    MemoryFts::delete().to_sql_for(Dialect::Sqlite).0,
    "DELETE FROM \"memory_fts\""
  );
}
