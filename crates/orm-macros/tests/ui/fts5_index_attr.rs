//! SQLite cannot index a virtual table, so `#[index]` on an FTS5 struct fails.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts")]
#[index("idx_memory_fts_body", body)]
pub struct MemoryFts {
  pub body: String,
}

fn main() {}
