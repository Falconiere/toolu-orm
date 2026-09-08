//! `#[fts5_table(...)]` with a multi-segment path as a key must fail.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts", fts5::tokenize = "porter")]
pub struct MemoryFts {
  pub body: String,
}

fn main() {}
