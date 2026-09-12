//! Duplicate `where = "..."` on `#[index]` must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index("idx_widgets_name", name, where = "deleted_at IS NULL", where = "name IS NOT NULL")]
pub struct Widget {
  pub id: String,
  pub name: String,
  pub deleted_at: Option<String>,
}

fn main() {}
