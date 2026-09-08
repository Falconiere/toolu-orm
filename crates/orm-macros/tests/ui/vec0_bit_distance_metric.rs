//! A `bit` vector has no `distance_metric`.

use toolu_orm_macros::vec0_table;

struct Text;
struct Vector;

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  #[column(dim = 1024, element = "bit", distance_metric = "cosine")]
  pub embedding: Vector,
}

fn main() {}
