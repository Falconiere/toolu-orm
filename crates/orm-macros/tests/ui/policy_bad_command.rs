//! `for = …` outside all/select/insert/update/delete must fail.

use toolu_orm_macros::table;

#[table(name = "docs")]
#[policy("tenant", for = truncate, using = "tenant_id = 1")]
pub struct Docs {
  pub id: String,
  pub tenant_id: i64,
}

fn main() {}
