//! `#[index(...)]` with a non-string first argument must fail.

use toolu_orm_macros::table;

#[table(name = "widgets")]
#[index(123)]
pub struct Widget {
  pub id: String,
}

fn main() {}
