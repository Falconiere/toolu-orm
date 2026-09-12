//! Tests for CREATE TABLE, DROP TABLE, STRICT mode, and compat type mapping.
//!
//! # Public API
//!
//! Tests: create table, drop table, strict mode, empty ops, non-strict compat types,
//! non-strict FK retention (references render regardless of `strict`).

use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

use super::column_helpers::{col, col_with_default};

#[test]
fn test_create_table_sql() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "conversations".to_owned(),
      columns: vec![
        col("id", ColumnType::Text, true, false),
        col("pipeline_id", ColumnType::Text, false, true),
        col_with_default("created_at", ColumnType::Integer, true, "unixepoch()"),
      ],
      indexes: vec![],
      primary_key: vec![],
      strict: false,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"conversations\""));
  assert!(sql.contains("\"id\" TEXT PRIMARY KEY"));
  assert!(sql.contains("\"pipeline_id\" TEXT NOT NULL"));
  assert!(sql.contains("\"created_at\" INTEGER NOT NULL DEFAULT (unixepoch())"));
}

#[test]
fn test_drop_table_sql() {
  let ops = vec![Operation::DropTable {
    name: "old_table".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("DROP TABLE IF EXISTS \"old_table\""));
}

#[test]
fn test_empty_operations_produces_empty_sql() {
  let sql = generate_sql_for(&[], Dialect::Sqlite);
  assert!(sql.is_empty());
}

#[test]
fn test_create_strict_table() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "users".to_owned(),
      columns: vec![col("id", ColumnType::Uuid, true, false)],
      indexes: vec![],
      primary_key: vec![],
      strict: true,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains(") STRICT;"), "expected STRICT, got: {sql}");
}

#[test]
fn test_non_strict_keeps_fk_references() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "messages".to_owned(),
      columns: vec![ColumnDef {
        name: "conversation_id".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: true,
        default: None,
        unique: false,
        references: Some("conversations(id)".to_owned()),
        on_delete: Some(ForeignKeyAction::Cascade),
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      }],
      indexes: vec![],
      primary_key: vec![],
      strict: false,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  // `references` is a foreign key regardless of `strict`; `strict` only
  // switches the SQLite column type set (README, "Defining tables").
  for dialect in [Dialect::Sqlite, Dialect::Postgres] {
    let sql = generate_sql_for(&ops, dialect);
    assert!(
      sql.contains(r#"REFERENCES "conversations"("id") ON DELETE CASCADE"#),
      "{dialect:?}: non-strict table must keep its FK clause, got: {sql}"
    );
  }
}

#[test]
fn test_non_strict_uses_compat_types() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "test".to_owned(),
      columns: vec![
        col("id", ColumnType::Uuid, true, false),
        col("is_active", ColumnType::Boolean, false, true),
        col("created_at", ColumnType::Timestamp, false, true),
        col("metadata", ColumnType::Json, false, false),
        col("age", ColumnType::SmallInt, false, false),
        col("big_num", ColumnType::BigInt, false, false),
        ColumnDef {
          name: "name".to_owned(),
          column_type: ColumnType::Varchar(100),
          primary_key: false,
          not_null: true,
          default: None,
          unique: false,
          references: None,
          on_delete: None,
          on_update: None,
          check: None,
          unindexed: false,
          autoincrement: false,
        },
      ],
      indexes: vec![],
      primary_key: vec![],
      strict: false,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  // Uuid, Varchar, Json should map to TEXT
  assert!(sql.contains(r#""id" TEXT"#), "Uuid -> TEXT, got: {sql}");
  assert!(
    sql.contains(r#""name" TEXT"#),
    "Varchar -> TEXT, got: {sql}"
  );
  assert!(
    sql.contains(r#""metadata" TEXT"#),
    "Json -> TEXT, got: {sql}"
  );
  // Boolean, Timestamp, SmallInt, BigInt should map to INTEGER
  assert!(
    sql.contains(r#""is_active" INTEGER"#),
    "Boolean -> INTEGER, got: {sql}"
  );
  assert!(
    sql.contains(r#""created_at" INTEGER"#),
    "Timestamp -> INTEGER, got: {sql}"
  );
  assert!(
    sql.contains(r#""age" INTEGER"#),
    "SmallInt -> INTEGER, got: {sql}"
  );
  assert!(
    sql.contains(r#""big_num" INTEGER"#),
    "BigInt -> INTEGER, got: {sql}"
  );
  // Should NOT have STRICT keyword
  assert!(!sql.contains("STRICT"), "non-strict table, got: {sql}");
}

#[test]
fn create_table_postgres_uses_uuid_and_boolean() {
  let table = TableDef {
    name: "users".to_owned(),
    columns: vec![
      ColumnDef {
        name: "id".to_owned(),
        column_type: ColumnType::Uuid,
        primary_key: true,
        not_null: true,
        default: Some("uuid4_str()".to_owned()),
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
      ColumnDef {
        name: "active".to_owned(),
        column_type: ColumnType::Boolean,
        primary_key: false,
        not_null: true,
        default: Some("true".to_owned()),
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
        autoincrement: false,
      },
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };

  let ops = vec![Operation::CreateTable { table }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert!(sql.contains("UUID"), "sql: {sql}");
  assert!(sql.contains("gen_random_uuid()"), "sql: {sql}");
  assert!(sql.contains("BOOLEAN"), "sql: {sql}");
  assert!(
    !sql.contains("\"active\" INTEGER"),
    "boolean should not map to INTEGER on Postgres: {sql}"
  );
}

#[test]
fn rename_table_sql() {
  let ops = vec![Operation::RenameTable {
    old: "users".to_owned(),
    new: "accounts".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(sql.trim(), "ALTER TABLE \"users\" RENAME TO \"accounts\";");
}

#[test]
fn rename_column_sql() {
  let ops = vec![Operation::RenameColumn {
    table: "users".to_owned(),
    old: "name".to_owned(),
    new: "full_name".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Postgres);
  assert_eq!(
    sql.trim(),
    "ALTER TABLE \"users\" RENAME COLUMN \"name\" TO \"full_name\";"
  );
}
