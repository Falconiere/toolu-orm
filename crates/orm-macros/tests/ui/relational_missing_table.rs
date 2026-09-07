//! `#[derive(Relational)]` without `#[relational(table = "...")]` must fail.

use toolu_orm_macros::Relational;

#[derive(Relational)]
struct Widget {
  pub id: String,
}

fn main() {
  let _ = Widget { id: String::new() };
}
