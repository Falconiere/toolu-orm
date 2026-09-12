use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::{ColumnChange, Operation};
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

use super::column_helpers::col;

#[test]
fn alter_column_postgres_uses_alter_table() {
  let table_def = TableDef {
    name: "users".to_owned(),
    columns: vec![],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };
  let ops = vec![Operation::AlterColumn {
    table: "users".to_owned(),
    changes: vec![
      ColumnChange::Type {
        column: "age".to_owned(),
        old: ColumnType::Integer,
        new: ColumnType::BigInt,
      },
      ColumnChange::Nullable {
        column: "age".to_owned(),
        old: true,
        new: false,
      },
    ],
    table_def,
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(
    sql.contains(r#"ALTER TABLE "users" ALTER COLUMN "age" TYPE BIGINT"#),
    "sql: {sql}"
  );
  assert!(
    sql.contains(r#"ALTER TABLE "users" ALTER COLUMN "age" SET NOT NULL"#),
    "sql: {sql}"
  );
  assert!(!sql.contains("PRAGMA"), "sql: {sql}");
}

#[test]
fn alter_column_sqlite_uses_table_recreation_path() {
  let table_def = TableDef {
    name: "users".to_owned(),
    columns: vec![
      col("id", ColumnType::Text, true, false),
      col("age", ColumnType::BigInt, false, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };
  let ops = vec![Operation::AlterColumn {
    table: "users".to_owned(),
    changes: vec![ColumnChange::Type {
      column: "age".to_owned(),
      old: ColumnType::Integer,
      new: ColumnType::BigInt,
    }],
    table_def,
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("PRAGMA foreign_keys = OFF"), "sql: {sql}");
  assert!(sql.contains("RENAME TO"), "sql: {sql}");
}
