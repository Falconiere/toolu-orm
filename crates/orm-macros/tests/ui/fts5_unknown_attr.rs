//! `#[fts5_table(...)]` with a key the FTS5 module does not take must fail.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts", strict = "yes")]
pub struct MemoryFts {
  pub body: String,
}

fn main() {}
