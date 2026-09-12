use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableDef;

fn make_table(name: &str) -> TableDef {
  TableDef {
    name: name.to_owned(),
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
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }
}

#[test]
fn test_registry_from_tables() {
  let registry =
    SchemaRegistry::from_tables(vec![make_table("conversations"), make_table("messages")]);
  assert_eq!(registry.tables().len(), 2);
}

#[test]
fn test_registry_find_table() {
  let registry = SchemaRegistry::from_tables(vec![make_table("conversations")]);
  assert!(registry.find_table("conversations").is_some());
  assert!(registry.find_table("nonexistent").is_none());
}

#[test]
fn test_registry_table_names_sorted() {
  let registry =
    SchemaRegistry::from_tables(vec![make_table("messages"), make_table("conversations")]);
  let names: Vec<&str> = registry.tables().iter().map(|t| t.name.as_str()).collect();
  assert_eq!(names, vec!["conversations", "messages"]);
}
