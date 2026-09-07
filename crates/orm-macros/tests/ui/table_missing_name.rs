//! `#[table]` without a `name` attribute must fail.

use toolu_orm_macros::table;

#[table]
pub struct Widget {
  pub id: String,
}

fn main() {}
