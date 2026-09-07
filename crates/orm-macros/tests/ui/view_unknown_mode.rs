//! `#[view(...)]` with an unknown mode must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[view(WidgetView, drop(id))]
pub struct Widget {
  pub id: String,
}

fn main() {}
