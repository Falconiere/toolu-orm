//! `vec0` cannot quote identifiers, so a hostile table name fails the build.

use toolu_orm_macros::vec0_table;

struct Text;
struct Vector;

#[vec0_table(name = "my table")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  #[column(dim = 8)]
  pub embedding: Vector,
}

fn main() {}
