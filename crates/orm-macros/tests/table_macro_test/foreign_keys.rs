//! Table-level `#[foreign_key(...)]` lands in `TableDef.foreign_keys`.

use toolu_orm_core::column::{ForeignKeyAction, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::snapshot::ForeignKeyDef;
use toolu_orm_core::sql::generate_sql_for;
use toolu_orm_core::table::TableSchema;
use toolu_orm_macros::table;

#[table(name = "project_work_items")]
#[unique_index("project_work_items_id_project_uidx", id, project_id)]
#[foreign_key(
  columns(parent_work_item_id, project_id),
  references = "project_work_items(id, project_id)"
)]
pub struct ProjectWorkItem {
  #[column(primary_key, not_null)]
  pub id: Text,
  #[column(not_null, references = "projects(id)", on_delete = "cascade")]
  pub project_id: Text,
  pub parent_work_item_id: Text,
}

#[table(name = "project_evidence")]
#[foreign_key(
  name = "project_evidence_work_item_project_fk",
  columns(work_item_id, project_id),
  references = "project_work_items( id , project_id )",
  on_delete = "cascade",
  on_update = "restrict"
)]
pub struct ProjectEvidence {
  #[column(primary_key, not_null)]
  pub id: Text,
  #[column(not_null)]
  pub project_id: Text,
  #[column(not_null)]
  pub work_item_id: Text,
}

fn key(name: &str, columns: [&str; 2], refs: [&str; 2]) -> ForeignKeyDef {
  ForeignKeyDef {
    name: name.to_owned(),
    columns: columns.map(str::to_owned).to_vec(),
    references_table: "project_work_items".to_owned(),
    references_columns: refs.map(str::to_owned).to_vec(),
    on_delete: None,
    on_update: None,
  }
}

#[test]
fn self_reference_takes_the_default_name_and_keeps_the_column_fk() -> Result<(), String> {
  let def = ProjectWorkItem::table_def();
  assert_eq!(
    def.foreign_keys,
    vec![key(
      "fk_project_work_items_parent_work_item_id_project_id",
      ["parent_work_item_id", "project_id"],
      ["id", "project_id"],
    )]
  );
  let project_id = def.find_column("project_id").ok_or("no project_id")?;
  assert_eq!(project_id.references.as_deref(), Some("projects(id)"));
  Ok(())
}

#[test]
fn explicit_name_and_actions_land_in_the_def() {
  let mut expected = key(
    "project_evidence_work_item_project_fk",
    ["work_item_id", "project_id"],
    ["id", "project_id"],
  );
  expected.on_delete = Some(ForeignKeyAction::Cascade);
  expected.on_update = Some(ForeignKeyAction::Restrict);
  let def = ProjectEvidence::table_def();
  assert_eq!(def.foreign_keys, vec![expected]);
  let sql = generate_sql_for(&[Operation::CreateTable { table: def }], Dialect::Sqlite);
  assert!(
    sql.contains(r#"FOREIGN KEY ("work_item_id", "project_id") REFERENCES "project_work_items" ("id", "project_id") ON DELETE CASCADE ON UPDATE RESTRICT"#),
    "{sql}"
  );
}
