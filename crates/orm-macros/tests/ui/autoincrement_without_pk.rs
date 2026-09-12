//! `autoincrement` without `primary_key` must fail.

use toolu_orm_macros::table;

#[table(name = "retrieval_log")]
pub struct RetrievalLog {
  #[column(autoincrement)]
  pub id: i64,
}

fn main() {}
