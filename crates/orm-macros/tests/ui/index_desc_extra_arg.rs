//! `desc()` with an extra argument must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index("idx_widgets_at", desc(at, extra))]
pub struct Widget {
  pub id: String,
  pub at: String,
}

fn main() {}
