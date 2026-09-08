//! Tests for CREATE INDEX, DROP INDEX, unique index, and VARCHAR column SQL.
//!
//! # Public API
//!
//! Tests: create index, create unique index, drop index, varchar column type.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

#[test]
fn test_create_index_sql() {
  let ops = vec![Operation::CreateIndex {
    table: "pipelines".to_owned(),
    index: IndexDef {
      name: "idx_pipelines_repo_id".to_owned(),
      columns: vec!["repo_id".to_owned()],
      unique: false,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql
      .contains(r#"CREATE INDEX IF NOT EXISTS "idx_pipelines_repo_id" ON "pipelines" ("repo_id")"#),
    "got: {sql}"
  );
}

#[test]
fn test_create_unique_index_sql() {
  let ops = vec![Operation::CreateIndex {
    table: "pipelines".to_owned(),
    index: IndexDef {
      name: "idx_unique_name".to_owned(),
      columns: vec!["repo_id".to_owned(), "name".to_owned()],
      unique: true,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(
      r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_unique_name" ON "pipelines" ("repo_id", "name")"#
    ),
    "got: {sql}"
  );
}

#[test]
fn test_drop_index_sql() {
  let ops = vec![Operation::DropIndex {
    name: "idx_old".to_owned(),
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(r#"DROP INDEX IF EXISTS "idx_old""#),
    "got: {sql}"
  );
}

#[test]
fn test_varchar_column_sql() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "users".to_owned(),
      columns: vec![ColumnDef {
        name: "email".to_owned(),
        column_type: ColumnType::Varchar(255),
        primary_key: false,
        not_null: true,
        default: None,
        unique: true,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
        unindexed: false,
      }],
      indexes: vec![],
      strict: true,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(r#""email" varchar(255) NOT NULL UNIQUE"#),
    "got: {sql}"
  );
}
