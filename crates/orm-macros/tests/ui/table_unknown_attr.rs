//! `#[table(...)]` with an attribute other than `name`/`strict` must fail.

use toolu_orm_macros::table;

#[table(name = "widgets", foo = "x")]
pub struct Widget {
  pub id: String,
}

fn main() {}
