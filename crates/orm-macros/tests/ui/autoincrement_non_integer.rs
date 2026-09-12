//! `autoincrement` on a non-Integer column must fail.

use toolu_orm_macros::table;

#[table(name = "retrieval_log")]
pub struct RetrievalLog {
  #[column(primary_key, autoincrement)]
  pub id: String,
}

fn main() {}
