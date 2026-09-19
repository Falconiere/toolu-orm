//! The result alias both suites return, and the sample schema the journal
//! suite diffs against.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::{TableDef, TableKind};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
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
    unindexed: false,
    autoincrement: false,
  }
}

pub fn sample_registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![TableDef {
    name: "conversations".to_owned(),
    columns: vec![
      col("id", ColumnType::Text, true, false),
      col("title", ColumnType::Text, false, false),
    ],
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
  }])
}
