//! A vec0 primary key is text or integer, never a floating type.

use toolu_orm_macros::vec0_table;

struct Real;
struct Vector;

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Real,
  #[column(dim = 8)]
  pub embedding: Vector,
}

fn main() {}
