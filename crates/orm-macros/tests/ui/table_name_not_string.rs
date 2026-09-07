//! `#[table(name = ...)]` with a non-string literal must fail.

use toolu_orm_macros::table;

#[table(name = 1)]
pub struct Widget {
  pub id: String,
}

fn main() {}
