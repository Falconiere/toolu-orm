//! `rls = …` outside "enable" / "force" must fail.

use toolu_orm_macros::table;

#[table(name = "docs", rls = "on")]
pub struct Docs {
  pub id: String,
}

fn main() {}
