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
  assert!(
    sql.contains("DROP TABLE IF EXISTS \"memory_fts\";"),
    "missing drop: {sql}"
  );
  assert!(
    sql.contains("CREATE VIRTUAL TABLE IF NOT EXISTS \"memory_fts\" USING \"fts5\""),
    "missing create: {sql}"
  );
  assert!(
    sql.contains("INSERT INTO \"memory_fts\"(\"memory_fts\") VALUES('rebuild');"),
    "missing rebuild: {sql}"
  );
  assert!(
    sql.contains("--> statement-breakpoint"),
    "missing breakpoints: {sql}"
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
  assert!(
    sql.contains("SQLite-only") && sql.contains("memory_fts"),
    "expected skip comment, got: {sql}"
  );
  assert!(
    !sql.contains("VALUES('rebuild')"),
    "postgres must not emit rebuild: {sql}"
  );
}
