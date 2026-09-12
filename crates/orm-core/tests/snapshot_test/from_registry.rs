use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::TableDef;

#[test]
fn from_registry_extracts_foreign_keys() -> Result<(), Box<dyn std::error::Error>> {
  let table = TableDef {
    name: "posts".to_owned(),
    columns: vec![
      ColumnDef {
        name: "id".to_owned(),
        column_type: ColumnType::Text,
        primary_key: true,
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
      ColumnDef {
        name: "author_id".to_owned(),
        column_type: ColumnType::Text,
        primary_key: false,
        not_null: true,
        default: None,
        unique: false,
        references: Some("users(id)".to_owned()),
        on_delete: Some(ForeignKeyAction::Cascade),
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

  let registry = SchemaRegistry::from_tables(vec![table]);
  let snapshot = Snapshot::from_registry(&registry);

  let posts = snapshot.tables.get("posts").ok_or("missing posts table")?;
  assert_eq!(posts.foreign_keys.len(), 1);
  let fk = posts.foreign_keys.values().next().ok_or("missing fk")?;
  assert_eq!(fk.columns, vec!["author_id"]);
  assert_eq!(fk.references_table, "users");
  assert_eq!(fk.references_columns, vec!["id"]);
  assert_eq!(fk.on_delete, Some(ForeignKeyAction::Cascade));
  Ok(())
}

#[test]
fn from_registry_columns_are_btreemap_keyed_by_name() -> Result<(), Box<dyn std::error::Error>> {
  let table = TableDef {
    name: "users".to_owned(),
    columns: vec![
      ColumnDef {
        name: "id".to_owned(),
        column_type: ColumnType::Text,
        primary_key: true,
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
      ColumnDef {
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
        autoincrement: false,
      },
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };

  let registry = SchemaRegistry::from_tables(vec![table]);
  let snapshot = Snapshot::from_registry(&registry);
  let users = snapshot.tables.get("users").ok_or("missing users table")?;

  assert_eq!(users.columns.len(), 2);
  assert!(users.columns.contains_key("id"));
  assert!(users.columns.contains_key("email"));
  assert!(
    users
      .columns
      .get("id")
      .ok_or("missing id column")?
      .primary_key
  );
  Ok(())
}

#[test]
fn from_registry_extracts_check_constraints() -> Result<(), Box<dyn std::error::Error>> {
  let table = TableDef {
    name: "items".to_owned(),
    columns: vec![ColumnDef {
      name: "status".to_owned(),
      column_type: ColumnType::Text,
      primary_key: false,
      not_null: true,
      default: None,
      unique: false,
      references: None,
      on_delete: None,
      on_update: None,
      check: Some("CHECK(\"status\" IN ('draft', 'active'))".to_owned()),
      unindexed: false,
      autoincrement: false,
    }],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  };

  let registry = SchemaRegistry::from_tables(vec![table]);
  let snapshot = Snapshot::from_registry(&registry);
  let items = snapshot.tables.get("items").ok_or("missing items table")?;

  assert_eq!(items.check_constraints.len(), 1);
  assert!(items.check_constraints.contains_key("status"));
  Ok(())
}

#[test]
fn from_registry_generates_unique_ids() {
  let registry = SchemaRegistry::from_tables(vec![]);
  let snap1 = Snapshot::from_registry(&registry);
  let snap2 = Snapshot::from_registry(&registry);

  assert_ne!(snap1.id, snap2.id);
  assert!(!snap1.id.is_empty());
  assert_eq!(snap1.prev_id, "00000000-0000-0000-0000-000000000000");
}
