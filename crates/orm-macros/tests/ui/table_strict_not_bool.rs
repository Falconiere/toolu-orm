//! `#[table(strict = ...)]` with a non-literal value must fail.

use toolu_orm_macros::table;

#[table(name = "widgets", strict = maybe)]
pub struct Widget {
  pub id: String,
}

fn main() {}
