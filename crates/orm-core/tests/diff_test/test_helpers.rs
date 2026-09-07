//! Shared helper constructors for diff tests.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::table::TableDef;

pub(crate) fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
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
  }
}

pub(crate) fn table(name: &str, columns: Vec<ColumnDef>) -> TableDef {
  TableDef {
    name: name.to_owned(),
    columns,
    indexes: vec![],
    strict: false,
    kind: toolu_orm_core::table::TableKind::Ordinary,
  }
}
