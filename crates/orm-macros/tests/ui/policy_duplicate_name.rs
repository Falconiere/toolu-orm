//! Two `#[policy]` attributes with the same name on one table must fail.

use toolu_orm_macros::table;

#[table(name = "docs")]
#[policy("tenant", using = "tenant_id = 1")]
#[policy("tenant", using = "tenant_id = 2")]
pub struct Docs {
  pub id: String,
  pub tenant_id: i64,
}

fn main() {}
