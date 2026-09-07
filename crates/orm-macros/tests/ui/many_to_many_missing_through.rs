//! `#[many_to_many(...)]` without `through` must fail.

use toolu_orm_macros::Relational;

#[derive(Relational)]
#[relational(table = "posts")]
struct Post {
  pub id: String,
  #[many_to_many(table = "tags", local_key = "post_id", foreign_key = "tag_id", columns = ["id"])]
  pub tags: Vec<Tag>,
}

#[derive(Debug, Clone)]
struct Tag {
  pub id: String,
}

fn main() {
  let _ = Post {
    id: String::new(),
    tags: vec![],
  };
  let _ = Tag { id: String::new() };
}
