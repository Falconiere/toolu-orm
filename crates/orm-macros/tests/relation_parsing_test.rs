use toolu_orm_macros::Relational;

/// Verifies `#[has_many]` parses on a `Vec` field.
#[derive(Relational)]
#[relational(table = "users")]
struct UserWithPosts {
  pub id: String,
  pub name: String,
  #[has_many(
    table = "posts",
    foreign_key = "author_id",
    columns = ["id", "title"]
  )]
  pub posts: Vec<PostRow>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PostRow {
  pub id: String,
  pub title: String,
}

/// Verifies `#[belongs_to]` parses on an `Option` field.
#[derive(Relational)]
#[relational(table = "posts")]
struct PostWithAuthor {
  pub id: String,
  pub title: String,
  #[belongs_to(
    table = "users",
    foreign_key = "author_id",
    columns = ["id", "name"]
  )]
  pub author: Option<AuthorRow>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AuthorRow {
  pub id: String,
  pub name: String,
}

/// Verifies `#[many_to_many]` parses with through table.
#[derive(Relational)]
#[relational(table = "posts")]
struct PostWithTags {
  pub id: String,
  #[many_to_many(
    table = "tags",
    through = "post_tags",
    local_key = "post_id",
    foreign_key = "tag_id",
    columns = ["id", "name"]
  )]
  pub tags: Vec<TagRow>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TagRow {
  pub id: String,
  pub name: String,
}

#[test]
fn has_many_attribute_compiles() {
  assert_eq!(UserWithPosts::RELATIONAL_TABLE, "users");
}

#[test]
fn belongs_to_attribute_compiles() {
  assert_eq!(PostWithAuthor::RELATIONAL_TABLE, "posts");
}

#[test]
fn many_to_many_attribute_compiles() {
  assert_eq!(PostWithTags::RELATIONAL_TABLE, "posts");
}

#[test]
fn has_many_field_is_vec_type() {
  fn assert_vec<T: IntoIterator>(_: &T) {}
  let user = UserWithPosts {
    id: "1".to_owned(),
    name: "test".to_owned(),
    posts: vec![],
  };
  assert_vec(&user.posts);
  assert_eq!(user.id.len() + user.name.len(), 5);
}

#[test]
fn belongs_to_field_is_option_type() {
  fn assert_option<T>(_: &Option<T>) {}
  let post = PostWithAuthor {
    id: "1".to_owned(),
    title: "test".to_owned(),
    author: None,
  };
  assert_option(&post.author);
  assert_eq!(post.id.len() + post.title.len(), 5);
}

#[test]
fn multiple_relation_attributes_on_same_struct() {
  #[derive(Relational)]
  #[relational(table = "users")]
  struct UserFull {
    pub id: String,
    #[has_many(
      table = "posts",
      foreign_key = "author_id",
      columns = ["id", "title"]
    )]
    pub posts: Vec<PostRow>,
    #[has_many(
      table = "comments",
      foreign_key = "user_id",
      columns = ["id", "body"]
    )]
    pub comments: Vec<CommentRow>,
  }

  #[derive(Debug, Clone, serde::Deserialize)]
  struct CommentRow {
    pub id: String,
    pub body: String,
  }

  assert_eq!(UserFull::RELATIONAL_TABLE, "users");
  let uf = UserFull {
    id: "i".to_owned(),
    posts: vec![],
    comments: vec![],
  };
  assert_eq!(uf.id.len(), 1);
  assert!(uf.posts.is_empty() && uf.comments.is_empty());
  let c = CommentRow {
    id: "i".to_owned(),
    body: "b".to_owned(),
  };
  assert_eq!(c.id.len() + c.body.len(), 2);
}

#[test]
fn row_helper_types_are_constructible() {
  let p = PostRow {
    id: "i".to_owned(),
    title: "t".to_owned(),
  };
  assert_eq!(p.id.len() + p.title.len(), 2);
  let a = AuthorRow {
    id: "i".to_owned(),
    name: "n".to_owned(),
  };
  assert_eq!(a.id.len() + a.name.len(), 2);
  let t = TagRow {
    id: "i".to_owned(),
    name: "n".to_owned(),
  };
  assert_eq!(t.id.len() + t.name.len(), 2);
  let pt = PostWithTags {
    id: "i".to_owned(),
    tags: vec![],
  };
  assert_eq!(pt.id.len(), 1);
  assert!(pt.tags.is_empty());
}
