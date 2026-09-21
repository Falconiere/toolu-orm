//! `#[policy]` on an FTS5 virtual table must fail.

use toolu_orm_macros::fts5_table;

#[fts5_table(name = "docs_fts")]
#[policy("tenant", using = "true")]
pub struct DocsFts {
  pub body: String,
}

fn main() {}
