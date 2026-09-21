//! Shared constructors: a `docs` table with or without row security, and the
//! diff of two versions of it.

use toolu_orm_core::column::{ColumnDef, ColumnType};
use toolu_orm_core::diff::{diff, Operation};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::policy::{PolicyDef, RowSecurity};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::table::{TableDef, TableKind};

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
    autoincrement: false,
  }
}

pub(crate) fn table(name: &str, columns: Vec<ColumnDef>) -> TableDef {
  TableDef {
    name: name.to_owned(),
    columns,
    indexes: vec![],
    primary_key: vec![],
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }
}

pub(crate) fn tenant_policy() -> PolicyDef {
  PolicyDef::new("tenant_isolation").using("tenant_id = current_setting('app.tenant_id')::int")
}

pub(crate) fn docs(security: Option<RowSecurity>) -> TableDef {
  let mut t = table(
    "docs",
    vec![
      col("id", ColumnType::Text, true, true),
      col("tenant_id", ColumnType::Integer, false, true),
    ],
  );
  t.row_security = security;
  t
}

pub(crate) fn snapshot_of(t: TableDef) -> Snapshot {
  Snapshot::from_registry(&SchemaRegistry::from_tables(vec![t]))
}

pub(crate) fn ops_between(old: TableDef, new: TableDef) -> Result<Vec<Operation>, DbCoreError> {
  diff(&snapshot_of(old), &SchemaRegistry::from_tables(vec![new]))
}
