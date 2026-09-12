use toolu_orm_core::column::ColumnDef;
use toolu_orm_core::column::ColumnType;

pub(super) fn col(name: &str, ct: ColumnType, pk: bool, nn: bool) -> ColumnDef {
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

pub(super) fn col_with_default(name: &str, ct: ColumnType, nn: bool, default: &str) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ct,
    primary_key: false,
    not_null: nn,
    default: Some(default.to_owned()),
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
    autoincrement: false,
  }
}
