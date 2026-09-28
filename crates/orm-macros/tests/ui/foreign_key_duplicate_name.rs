//! Two foreign keys on one table cannot share a name.

use toolu_orm_macros::table;

#[table(name = "evidence")]
#[foreign_key(name = "evidence_fk", columns(work_item_id, project_id), references = "work_items(id, project_id)")]
#[foreign_key(name = "evidence_fk", columns(run_id, project_id), references = "runs(id, project_id)")]
pub struct Evidence {
  pub work_item_id: String,
  pub run_id: String,
  pub project_id: String,
}

fn main() {}
