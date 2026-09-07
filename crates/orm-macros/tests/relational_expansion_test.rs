use toolu_orm_core::relational_row::FromRelationalRow;
use toolu_orm_macros::Relational;

#[derive(Debug, Clone, serde::Deserialize)]
struct PostSummary {
  pub id: String,
  pub title: String,
}

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
  pub posts: Vec<PostSummary>,
}

#[test]
fn scalar_columns_lists_non_relation_fields() {
  assert_eq!(UserWithPosts::SCALAR_COLUMNS, &["id", "name"]);
}

#[test]
fn from_relational_values_deserializes_scalars_and_many() -> Result<(), Box<dyn std::error::Error>>
{
  let values = vec![
    serde_json::json!("user-1"),
    serde_json::json!("Alice"),
    serde_json::json!([["post-1", "Hello"], ["post-2", "World"]]),
  ];
  let user = UserWithPosts::from_relational_values(&values)?;
  assert_eq!(user.id, "user-1");
  assert_eq!(user.name, "Alice");
  assert_eq!(user.posts.len(), 2);
  let p0 = user.posts.first().ok_or("missing p0")?;
  let p1 = user.posts.get(1).ok_or("missing p1")?;
  assert_eq!(p0.id, "post-1");
  assert_eq!(p0.title, "Hello");
  assert_eq!(p1.id, "post-2");
  assert_eq!(p1.title, "World");
  Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AuthorRow {
  pub id: String,
  pub name: String,
}

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

#[test]
fn from_relational_values_deserializes_belongs_to() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("post-1"),
    serde_json::json!("Hello"),
    serde_json::json!(["user-1", "Alice"]),
  ];
  let post = PostWithAuthor::from_relational_values(&values)?;
  assert_eq!(post.id, "post-1");
  assert_eq!(post.title, "Hello");
  let author = post.author.as_ref().ok_or("missing author")?;
  assert_eq!(author.id, "user-1");
  assert_eq!(author.name, "Alice");
  Ok(())
}

#[test]
fn from_relational_values_handles_null_belongs_to() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("post-1"),
    serde_json::json!("Orphan post"),
    serde_json::json!(null),
  ];
  let post = PostWithAuthor::from_relational_values(&values)?;
  assert!(post.author.is_none());
  Ok(())
}

#[test]
fn from_relational_values_handles_empty_has_many() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("user-1"),
    serde_json::json!("Alice"),
    serde_json::json!([]),
  ];
  let user = UserWithPosts::from_relational_values(&values)?;
  assert_eq!(user.posts.len(), 0);
  Ok(())
}
