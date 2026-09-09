//! SQL for `Operation::RecreateFts5FromContent`.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::sql::generate_sql_for;

fn external_fts() -> toolu_orm_core::table::TableDef {
  Fts5Table::new("memory_fts")
    .column("body", ColumnType::Text)
    .tokenize("unicode61")
    .content("memories")
    .content_rowid("id")
    .build()
}

#[test]
fn recreate_fts5_from_content_sqlite_emits_drop_create_rebuild() {
  let sql = generate_sql_for(
    &[Operation::RecreateFts5FromContent {
      table: external_fts(),
    }],
    Dialect::Sqlite,
  );
  assert_eq!(
    sql.trim(),
    "DROP TABLE IF EXISTS \"memory_fts\";\n\
     --> statement-breakpoint\n\
     CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\" USING \"fts5\"(\"body\", tokenize = 'unicode61', content = 'memories', content_rowid = 'id');\n\
     --> statement-breakpoint\n\
     INSERT INTO \"memory_fts\"(\"memory_fts\") VALUES('rebuild');"
  );
}

#[test]
fn recreate_fts5_from_content_postgres_skips_without_rebuild() {
  let sql = generate_sql_for(
    &[Operation::RecreateFts5FromContent {
      table: external_fts(),
    }],
    Dialect::Postgres,
  );
  assert_eq!(
    sql.trim(),
    "-- virtual table \"memory_fts\" USING \"fts5\" is SQLite-only; skipped for postgres"
  );
  assert!(!sql.contains("VALUES('rebuild')"));
}
