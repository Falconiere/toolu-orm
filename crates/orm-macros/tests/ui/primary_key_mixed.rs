//! Mixing table-level and column-level primary keys must fail.

use toolu_orm_macros::table;

#[table(name = "memory_tags")]
#[primary_key(memory_id, tag)]
pub struct MemoryTags {
  #[column(primary_key)]
  pub memory_id: String,
  pub tag: String,
}

fn main() {}
