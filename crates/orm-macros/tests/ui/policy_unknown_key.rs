//! An unknown key in `#[policy]` must fail.

use toolu_orm_macros::table;

#[table(name = "docs")]
#[policy("tenant", check = "tenant_id = 1")]
pub struct Docs {
  pub id: String,
  pub tenant_id: i64,
}

fn main() {}
