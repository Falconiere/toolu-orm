//! The project-scoped schema of issue #264: work items that reference their
//! own table and evidence rows that reference work items, each through a
//! composite `(id, project_id)` foreign key.

use toolu_orm_core::column::{ColumnDef, ColumnType, ForeignKeyAction};
use toolu_orm_core::index::IndexDef;
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::ForeignKeyDef;
use toolu_orm_core::table::{TableDef, TableKind};

pub fn column(name: &str, primary_key: bool, not_null: bool) -> ColumnDef {
  ColumnDef {
    name: name.to_owned(),
    column_type: ColumnType::Text,
    primary_key,
    not_null,
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

pub fn table(name: &str, columns: Vec<ColumnDef>, foreign_keys: Vec<ForeignKeyDef>) -> TableDef {
  TableDef {
    name: name.to_owned(),
    columns,
    indexes: vec![],
    primary_key: vec![],
    foreign_keys,
    strict: false,
    kind: TableKind::Ordinary,
    fts5_sync: None,
    row_security: None,
  }
}

pub fn composite_fk(
  name: &str,
  columns: [&str; 2],
  references_table: &str,
  on_delete: Option<ForeignKeyAction>,
) -> ForeignKeyDef {
  ForeignKeyDef {
    name: name.to_owned(),
    columns: columns.iter().map(|c| (*c).to_owned()).collect(),
    references_table: references_table.to_owned(),
    references_columns: vec!["id".to_owned(), "project_id".to_owned()],
    on_delete,
    on_update: None,
  }
}

/// `(parent_work_item_id, project_id)` back into the same table, no action.
pub fn parent_fk() -> ForeignKeyDef {
  composite_fk(
    "fk_project_work_items_parent_work_item_id_project_id",
    ["parent_work_item_id", "project_id"],
    "project_work_items",
    None,
  )
}

/// `(work_item_id, project_id)` into the work items, cascading deletes.
pub fn evidence_fk() -> ForeignKeyDef {
  composite_fk(
    "project_evidence_work_item_project_fk",
    ["work_item_id", "project_id"],
    "project_work_items",
    Some(ForeignKeyAction::Cascade),
  )
}

pub fn projects() -> TableDef {
  table("projects", vec![column("id", true, true)], vec![])
}

/// Work items: the per-column FK to `projects` plus the composite self-FK,
/// and the unique `(id, project_id)` every composite FK here points at.
pub fn work_items() -> TableDef {
  let mut project_id = column("project_id", false, true);
  project_id.references = Some("projects(id)".to_owned());
  project_id.on_delete = Some(ForeignKeyAction::Cascade);
  let mut t = table(
    "project_work_items",
    vec![
      column("id", true, true),
      project_id,
      column("parent_work_item_id", false, false),
    ],
    vec![parent_fk()],
  );
  t.indexes = vec![IndexDef {
    name: "project_work_items_id_project_uidx".to_owned(),
    columns: vec!["id".into(), "project_id".into()],
    unique: true,
    where_clause: None,
  }];
  t
}

pub fn evidence(foreign_keys: Vec<ForeignKeyDef>) -> TableDef {
  table(
    "project_evidence",
    vec![
      column("id", true, true),
      column("project_id", false, true),
      column("work_item_id", false, true),
    ],
    foreign_keys,
  )
}

pub fn registry() -> SchemaRegistry {
  SchemaRegistry::from_tables(vec![
    projects(),
    work_items(),
    evidence(vec![evidence_fk()]),
  ])
}
