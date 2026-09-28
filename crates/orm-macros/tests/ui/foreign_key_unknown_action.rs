//! Actions use the `#[column]` vocabulary.

use toolu_orm_macros::table;

#[table(name = "evidence")]
#[foreign_key(
  columns(work_item_id, project_id),
  references = "work_items(id, project_id)",
  on_delete = "delete"
)]
pub struct Evidence {
  pub work_item_id: String,
  pub project_id: String,
}

fn main() {}
