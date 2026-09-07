//! `#[derive(Relational)]` on an enum must fail.

use toolu_orm_macros::Relational;

#[derive(Relational)]
#[relational(table = "widgets")]
enum Widget {
  A,
  B,
}

fn main() {
  let _ = Widget::A;
}
