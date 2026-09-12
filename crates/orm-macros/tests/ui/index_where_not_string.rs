//! `where =` with a non-string value must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index("idx_widgets_name", name, where = 123)]
pub struct Widget {
  pub id: String,
  pub name: String,
}

fn main() {}
