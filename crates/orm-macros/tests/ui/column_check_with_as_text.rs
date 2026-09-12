//! `check` and `as_text` are two sources of truth for a column CHECK; combining
//! them must fail at expand time.

use toolu_orm_macros::{table, ColumnEnum};

#[derive(ColumnEnum)]
enum Kind {
  Decision,
  Bug,
}

#[table(name = "items")]
pub struct Item {
  #[column(as_text, check = "kind IN ('decision','bug')")]
  pub kind: Kind,
}

fn main() {}
