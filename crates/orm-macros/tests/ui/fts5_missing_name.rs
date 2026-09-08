//! `#[fts5_table]` without `name` must fail.

use toolu_orm_macros::fts5_table;

#[fts5_table(tokenize = "porter")]
pub struct MemoryFts {
  pub body: String,
}

fn main() {}
