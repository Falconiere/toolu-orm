//! FTS5 columns carry no constraints, so a constraint must fail the build
//! rather than be silently dropped from the DDL — including next to the one
//! attribute that does apply, `unindexed`.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts")]
pub struct MemoryFts {
  #[column(unindexed, primary_key)]
  pub memory_id: String,
  pub body: String,
}

fn main() {}
