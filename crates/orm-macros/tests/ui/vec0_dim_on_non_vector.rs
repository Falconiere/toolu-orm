//! `dim` belongs only on a `Vector` column.

use toolu_orm_macros::vec0_table;

struct Text;

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  #[column(dim = 1024)]
  pub label: Text,
}

fn main() {}
