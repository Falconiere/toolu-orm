//! A single-column foreign key belongs on the column.

use toolu_orm_macros::table;

#[table(name = "evidence")]
#[foreign_key(columns(work_item_id), references = "work_items(id)")]
pub struct Evidence {
  pub work_item_id: String,
}

fn main() {}
