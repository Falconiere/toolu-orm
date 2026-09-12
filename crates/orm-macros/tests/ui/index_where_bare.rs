//! Bare `where` without `= "..."` must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index("idx_widgets_name", name, where)]
pub struct Widget {
  pub id: String,
  pub name: String,
}

fn main() {}
