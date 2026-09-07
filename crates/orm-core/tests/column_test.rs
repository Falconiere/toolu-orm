use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};

#[test]
fn test_column_def_with_all_constraints() {
  let col = ColumnDef {
    name: "id".to_owned(),
    column_type: ColumnType::Text,
    primary_key: true,
    not_null: false,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
  };
  assert_eq!(col.name, "id");
  assert_eq!(col.column_type, ColumnType::Text);
  assert!(col.primary_key);
}

#[test]
fn test_column_type_to_sql() {
  assert_eq!(ColumnType::Text.as_sql(), "TEXT");
  assert_eq!(ColumnType::Integer.as_sql(), "INTEGER");
  assert_eq!(ColumnType::Real.as_sql(), "REAL");
  assert_eq!(ColumnType::Blob.as_sql(), "BLOB");
}

#[test]
fn test_column_def_with_default_and_references() {
  let col = ColumnDef {
    name: "created_at".to_owned(),
    column_type: ColumnType::Integer,
    primary_key: false,
    not_null: true,
    default: Some("unixepoch()".to_owned()),
    unique: false,
    references: Some("users(id)".to_owned()),
    on_delete: None,
    on_update: None,
    check: None,
  };
  assert!(col.not_null);
  assert_eq!(col.default.as_deref(), Some("unixepoch()"));
  assert_eq!(col.references.as_deref(), Some("users(id)"));
}

#[test]
fn test_new_column_types_to_sql() {
  assert_eq!(ColumnType::Uuid.as_sql(), "uuid");
  assert_eq!(ColumnType::Boolean.as_sql(), "boolean");
  assert_eq!(ColumnType::Timestamp.as_sql(), "timestamp");
  assert_eq!(ColumnType::Date.as_sql(), "date");
  assert_eq!(ColumnType::Time.as_sql(), "time");
  assert_eq!(ColumnType::Json.as_sql(), "json");
  assert_eq!(ColumnType::BigInt.as_sql(), "bigint");
  assert_eq!(ColumnType::SmallInt.as_sql(), "smallint");
  assert_eq!(ColumnType::Varchar(255).as_sql(), "varchar(255)");
  assert_eq!(ColumnType::Varchar(50).as_sql(), "varchar(50)");
}

#[test]
fn test_foreign_key_action_as_sql() {
  assert_eq!(ForeignKeyAction::Cascade.as_sql(), "CASCADE");
  assert_eq!(ForeignKeyAction::SetNull.as_sql(), "SET NULL");
  assert_eq!(ForeignKeyAction::SetDefault.as_sql(), "SET DEFAULT");
  assert_eq!(ForeignKeyAction::Restrict.as_sql(), "RESTRICT");
  assert_eq!(ForeignKeyAction::NoAction.as_sql(), "NO ACTION");
}

#[test]
fn test_column_def_with_cascade() {
  let col = ColumnDef {
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
  };
  assert_eq!(col.on_delete, Some(ForeignKeyAction::Cascade));
}

#[test]
fn test_column_def_with_check() {
  let col = ColumnDef {
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
  };
  assert!(col.check.is_some());
}
