//! A non-`desc` call in an index column position must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index("idx_widgets_at", asc(at))]
pub struct Widget {
  pub id: String,
  pub at: String,
}

fn main() {}
