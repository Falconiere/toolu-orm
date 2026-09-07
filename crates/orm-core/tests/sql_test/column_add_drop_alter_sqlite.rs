use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{ColumnChange, Operation};
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

use super::column_helpers::col;

#[test]
fn test_add_column_sql() {
  let ops = vec![Operation::AddColumn {
    table: "conversations".to_owned(),
    column: col("title", ColumnType::Text, false, false),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("ALTER TABLE \"conversations\" ADD COLUMN \"title\" TEXT"));
}

#[test]
fn test_drop_column_sqlite_uses_alter_drop() {
  let ops = vec![Operation::DropColumn {
    table: "users".to_owned(),
    column: "legacy_field".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains(r#"DROP COLUMN "legacy_field""#), "sql: {sql}");
}

#[test]
fn test_alter_column_generates_table_recreation_sqlite() {
  let ops = vec![Operation::AlterColumn {
    table: "conversations".to_owned(),
    changes: vec![ColumnChange::Nullable {
      column: "title".to_owned(),
      old: true,
      new: false,
    }],
    table_def: TableDef {
      name: "conversations".to_owned(),
      columns: vec![
        col("id", ColumnType::Text, true, false),
        col("title", ColumnType::Text, false, true),
      ],
      indexes: vec![],
      strict: false,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("PRAGMA foreign_keys = OFF"), "sql: {sql}");
  assert!(
    sql.contains(r#"ALTER TABLE "conversations" RENAME TO "_conversations_old""#),
    "sql: {sql}"
  );
  assert!(
    sql.contains(r#"CREATE TABLE IF NOT EXISTS "conversations""#),
    "sql: {sql}"
  );
  assert!(sql.contains(r#"INSERT INTO "conversations""#), "sql: {sql}");
  assert!(
    sql.contains(r#"DROP TABLE "_conversations_old""#),
    "sql: {sql}"
  );
  assert!(sql.contains("PRAGMA foreign_keys = ON"), "sql: {sql}");
  assert!(sql.contains("--> statement-breakpoint"), "sql: {sql}");
  assert!(sql.contains("NOT NULL"), "sql: {sql}");
}
