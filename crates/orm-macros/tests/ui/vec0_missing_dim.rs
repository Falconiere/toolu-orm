//! A `Vector` field needs `#[column(dim = N)]`.

use toolu_orm_macros::vec0_table;

struct Text;
struct Vector;

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  pub embedding: Vector,
}

fn main() {}
