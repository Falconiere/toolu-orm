use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableDef;

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(crate) fn sample_registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![TableDef {
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
        unindexed: false,
        autoincrement: false,
      },
      ColumnDef {
        name: "created_at".to_owned(),
        column_type: ColumnType::Integer,
        primary_key: false,
        not_null: true,
        default: Some("unixepoch()".to_owned()),
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
  }])
}
