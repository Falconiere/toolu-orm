//! `references` must read `table(col, …)`.

use toolu_orm_macros::table;

#[table(name = "evidence")]
#[foreign_key(columns(work_item_id, project_id), references = "work_items")]
pub struct Evidence {
  pub work_item_id: String,
  pub project_id: String,
}

fn main() {}
