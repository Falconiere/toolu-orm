use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::table::TableDef;

fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ct,
    primary_key: pk,
    not_null: nn,
    default: None,
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
  }
}

#[test]
fn test_table_def_creation() {
  let table = TableDef {
    name: "conversations".to_owned(),
    columns: vec![
      ColumnDef {
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
      },
      ColumnDef {
        name: "title".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: false,
        default: None,
        unique: false,
        references: None,
        on_delete: None,
        on_update: None,
        check: None,
      },
    ],
    indexes: vec![],
    strict: false,
  };
  assert_eq!(table.name, "conversations");
  assert_eq!(table.columns.len(), 2);
}

#[test]
fn test_table_def_find_column() {
  let table = TableDef {
    name: "messages".to_owned(),
    columns: vec![ColumnDef {
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
    }],
    indexes: vec![],
    strict: false,
  };
  assert!(table.find_column("id").is_some());
  assert!(table.find_column("nonexistent").is_none());
}

#[test]
fn test_table_def_with_indexes_and_strict() -> Result<(), Box<dyn std::error::Error>> {
  let t = TableDef {
    name: "pipelines".to_owned(),
    columns: vec![col("id", ColumnType::Uuid, true, false)],
    indexes: vec![IndexDef {
      name: "idx_pipelines_repo_id".to_owned(),
      columns: vec!["repo_id".to_owned()],
      unique: false,
    }],
    strict: true,
  };
  assert!(t.strict);
  assert_eq!(t.indexes.len(), 1);
  let first = t.indexes.first().ok_or("expected one index")?;
  assert_eq!(first.name, "idx_pipelines_repo_id");
  Ok(())
}

#[test]
fn test_table_def_defaults_no_strict_no_indexes() {
  let t = TableDef {
    name: "legacy".to_owned(),
    columns: vec![],
    indexes: vec![],
    strict: false,
  };
  assert!(!t.strict);
  assert!(t.indexes.is_empty());
}
