use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableDef;

#[test]
fn test_column_with_unique_constraint() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "users".to_owned(),
      columns: vec![ColumnDef {
        name: "email".to_owned(),
        column_type: ColumnType::Text,
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
      strict: false,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(sql.contains("\"email\" TEXT NOT NULL UNIQUE"));
}

#[test]
fn test_column_with_references() {
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
  assert!(sql.contains("REFERENCES \"conversations\"(\"id\")"));
}

#[test]
fn test_create_table_with_on_delete_cascade() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "nodes".to_owned(),
      columns: vec![ColumnDef {
        name: "pipeline_id".to_owned(),
        column_type: ColumnType::Uuid,
        primary_key: false,
        not_null: true,
        default: None,
        unique: false,
        references: Some("pipelines(id)".to_owned()),
        on_delete: Some(ForeignKeyAction::Cascade),
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
    sql.contains("ON DELETE CASCADE"),
    "expected ON DELETE CASCADE, got: {sql}"
  );
  assert!(
    sql.contains(r#"REFERENCES "pipelines"("id")"#),
    "expected REFERENCES, got: {sql}"
  );
}

#[test]
fn test_create_table_with_check_constraint() {
  let ops = vec![Operation::CreateTable {
    table: TableDef {
      name: "pipelines".to_owned(),
      columns: vec![ColumnDef {
        name: "status".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: true,
        default: Some("'draft'".to_owned()),
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: Some(r#"CHECK("status" IN ('draft', 'active', 'archived'))"#.to_owned()),
        unindexed: false,
      }],
      indexes: vec![],
      strict: true,
      kind: toolu_orm_core::table::TableKind::Ordinary,
    },
  }];
  let sql = generate_sql_for(&ops, Dialect::Sqlite);
  assert!(
    sql.contains(r#"CHECK("status" IN ('draft', 'active', 'archived'))"#),
    "got: {sql}"
  );
}
