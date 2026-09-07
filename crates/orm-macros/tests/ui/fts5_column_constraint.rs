//! FTS5 columns carry no constraints, so `#[column(primary_key)]` must fail
//! rather than be silently dropped from the DDL.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts")]
pub struct MemoryFts {
  #[column(primary_key)]
  pub memory_id: String,
  pub body: String,
}

fn main() {}
