//! `TableKind::Virtual` inside orm-core: what the FTS5 builder renders, what
//! the DDL looks like on each dialect, and what survives a snapshot.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::{TableDef, TableKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The real table from the adopting project in issue #18.
fn memory_fts() -> TableDef {
  Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .column("tags", ColumnType::Text)
    .tokenize("porter unicode61 remove_diacritics 2")
    .build()
}

fn create_sql(table: &TableDef, dialect: Dialect) -> String {
  generate_sql_for(
    &[Operation::CreateTable {
      table: table.clone(),
    }],
    dialect,
  )
}

#[test]
fn builder_renders_columns_then_options() {
  let table = memory_fts();
  assert_eq!(table.kind.module(), Some("fts5"));
  assert_eq!(
    table.kind.args(),
    [
      "\"memory_id\" UNINDEXED",
      "\"body\"",
      "\"tags\"",
      "tokenize = 'porter unicode61 remove_diacritics 2'",
    ]
  );
}

#[test]
fn builder_marks_only_the_unindexed_column() {
  let table = memory_fts();
  let unindexed: Vec<&str> = table
    .columns
    .iter()
    .filter(|c| c.unindexed)
    .map(|c| c.name.as_str())
    .collect();
  assert_eq!(unindexed, ["memory_id"]);
  assert!(table.is_virtual());
  assert!(table.indexes.is_empty());
}

#[test]
fn builder_renders_every_option_in_a_fixed_order() {
  let table = Fts5Table::new("code_fts")
    .column("symbol", ColumnType::Text)
    .detail("none")
    .columnsize(0)
    .content_rowid("id")
    .content("symbols")
    .tokenize("identifier")
    .prefix("2 3")
    .build();
  assert_eq!(
    table.kind.args(),
    [
      "\"symbol\"",
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
fn builder_escapes_single_quotes_in_option_values() {
  let table = Fts5Table::new("quoted_fts")
    .column("body", ColumnType::Text)
    .tokenize("it's")
    .build();
  assert_eq!(table.kind.args(), ["\"body\"", "tokenize = 'it''s'"]);
}

#[test]
fn sqlite_ddl_creates_the_virtual_table() {
  let sql = create_sql(&memory_fts(), Dialect::Sqlite);
  assert_eq!(
    sql,
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\" USING \"fts5\"(\"memory_id\" UNINDEXED, \
     \"body\", \"tags\", tokenize = 'porter unicode61 remove_diacritics 2');"
  );
}

#[test]
fn sqlite_ddl_omits_types_strict_and_constraints() {
  let mut table = memory_fts();
  table.strict = true;
  if let Some(first) = table.columns.first_mut() {
    first.primary_key = true;
    first.not_null = true;
    first.default = Some("'x'".to_owned());
  }
  let sql = create_sql(&table, Dialect::Sqlite);
  assert!(!sql.contains("TEXT"), "column types leaked: {sql}");
  assert!(!sql.contains("STRICT"), "STRICT leaked: {sql}");
  assert!(!sql.contains("PRIMARY KEY"), "constraint leaked: {sql}");
  assert!(!sql.contains("DEFAULT"), "default leaked: {sql}");
}

#[test]
fn a_module_without_arguments_omits_the_parentheses() {
  let table = TableDef {
    name: "series".to_owned(),
    columns: vec![],
    indexes: vec![],
    strict: false,
    kind: TableKind::virtual_table("series", vec![]),
  };
  assert_eq!(
    create_sql(&table, Dialect::Sqlite),
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"series\" USING \"series\";"
  );
}

/// The module name is an identifier like any other, so it is quoted and its
/// quotes are doubled: it cannot end the statement and start a new one.
#[test]
fn a_module_name_cannot_end_the_statement() {
  let table = TableDef {
    name: "hostile".to_owned(),
    columns: vec![],
    indexes: vec![],
    strict: false,
    kind: TableKind::virtual_table("fts5\"); DROP TABLE users; --", vec![]),
  };
  assert_eq!(
    create_sql(&table, Dialect::Sqlite),
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"hostile\" USING \"fts5\"\"); DROP TABLE users; --\";"
  );
}

#[test]
fn postgres_reports_the_skipped_table_instead_of_emitting_ddl() {
  let sql = create_sql(&memory_fts(), Dialect::Postgres);
  assert_eq!(
    sql,
    "-- virtual table \"memory_fts\" USING \"fts5\" is SQLite-only; skipped for postgres"
  );
  assert!(!sql.contains("CREATE"), "emitted DDL for Postgres: {sql}");
}

#[test]
fn snapshot_round_trip_keeps_the_module_arguments() -> TestResult {
  let registry = SchemaRegistry::from_tables(vec![memory_fts()]);
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("virtual.snapshot.json");
  let path = path.to_str().ok_or("non-UTF8 path")?;
  Snapshot::from_registry(&registry).write_to_path(path)?;

  let loaded = Snapshot::read_from_path(path)?;
  let table = loaded.tables.get("memory_fts").ok_or("missing table")?;
  assert_eq!(table.kind, memory_fts().kind);
  assert!(
    table
      .columns
      .get("memory_id")
      .ok_or("missing memory_id")?
      .unindexed
  );

  let restored = loaded.to_registry();
  let restored = restored.find_table("memory_fts").ok_or("missing table")?;
  assert_eq!(restored, &memory_fts());
  Ok(())
}

#[test]
fn ordinary_tables_write_no_kind_key() -> TestResult {
  let mut ordinary = Fts5Table::new("memories")
    .column("body", ColumnType::Text)
    .build();
  ordinary.kind = TableKind::Ordinary;
  let registry = SchemaRegistry::from_tables(vec![ordinary]);
  let json = serde_json::to_string(&Snapshot::from_registry(&registry))?;
  assert!(!json.contains("\"kind\""), "kind leaked into JSON: {json}");
  assert!(
    !json.contains("\"unindexed\""),
    "unindexed leaked into JSON: {json}"
  );
  Ok(())
}
