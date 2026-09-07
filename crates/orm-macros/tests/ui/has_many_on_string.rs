//! `#[has_many(...)]` on a field that is neither `Vec<T>` nor `Option<T>` must fail.

use toolu_orm_macros::Relational;

#[derive(Relational)]
#[relational(table = "authors")]
struct Author {
  pub id: String,
  #[has_many(table = "posts", foreign_key = "author_id", columns = ["id"])]
  pub posts: String,
}

fn main() {
  let _ = Author {
    id: String::new(),
    posts: String::new(),
  };
}
