//! Integration tests: derive + type-state builder + both SQL dialects.

use toolu_orm_core::relational_row::{
  parse_json_array_of_arrays, FromRelationalRow, RelationDeserializer,
};
use toolu_orm_macros::Relational;
use toolu_orm_query::relational_builder::RelationalQuery;
use toolu_orm_query::select::RelationalSelectBuilder;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct PostSummary {
  pub id: String,
  pub title: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct AuthorRow {
  pub id: String,
  pub name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct CommentRow {
  pub id: String,
  pub body: String,
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

#[derive(Relational)]
#[relational(table = "users")]
struct UserWithMultipleRelations {
  pub id: String,
  pub name: String,
  #[has_many(
    table = "posts",
    foreign_key = "author_id",
    columns = ["id", "title"]
  )]
  pub posts: Vec<PostSummary>,
  #[has_many(
    table = "comments",
    foreign_key = "user_id",
    columns = ["id", "body"]
  )]
  pub comments: Vec<CommentRow>,
}

#[test]
fn derive_macro_scalar_columns_are_correct() -> Result<(), Box<dyn std::error::Error>> {
  assert_eq!(UserWithPosts::SCALAR_COLUMNS, &["id", "name"]);
  assert_eq!(PostWithAuthor::SCALAR_COLUMNS, &["id", "title"]);
  assert_eq!(UserWithMultipleRelations::SCALAR_COLUMNS, &["id", "name"]);
  assert_eq!(UserWithPosts::RELATIONAL_TABLE, "users");
  assert_eq!(PostWithAuthor::RELATIONAL_TABLE, "posts");
  assert_eq!(UserWithMultipleRelations::RELATIONAL_TABLE, "users");

  let values = vec![
    serde_json::json!("u1"),
    serde_json::json!("n"),
    serde_json::json!([]),
    serde_json::json!([]),
  ];
  let u = UserWithMultipleRelations::from_relational_values(&values)?;
  assert_eq!(u.id, "u1");
  assert_eq!(u.name, "n");
  assert!(u.posts.is_empty());
  assert!(u.comments.is_empty());
  Ok(())
}

#[test]
fn derive_macro_has_many_deserialization() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("user-1"),
    serde_json::json!("Alice"),
    serde_json::json!([["post-1", "First"], ["post-2", "Second"]]),
  ];
  let user = UserWithPosts::from_relational_values(&values)?;
  assert_eq!(user.id, "user-1");
  assert_eq!(user.name, "Alice");
  assert_eq!(user.posts.len(), 2);
  let p0 = user.posts.first().ok_or("missing p0")?;
  let p1 = user.posts.get(1).ok_or("missing p1")?;
  assert_eq!(p0.id, "post-1");
  assert_eq!(p1.title, "Second");
  Ok(())
}

#[test]
fn derive_macro_belongs_to_deserialization() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("post-1"),
    serde_json::json!("My Post"),
    serde_json::json!({"id": "user-1", "name": "Alice"}),
  ];
  let post = PostWithAuthor::from_relational_values(&values)?;
  assert_eq!(post.id, "post-1");
  assert_eq!(post.title, "My Post");
  let author = post.author.as_ref().ok_or("missing author")?;
  assert_eq!(author.id, "user-1");
  assert_eq!(author.name, "Alice");
  Ok(())
}

#[test]
fn derive_macro_null_belongs_to_deserialization() -> Result<(), Box<dyn std::error::Error>> {
  let values = vec![
    serde_json::json!("post-1"),
    serde_json::json!("Orphan"),
    serde_json::json!(null),
  ];
  let post = PostWithAuthor::from_relational_values(&values)?;
  assert!(post.author.is_none());
  Ok(())
}

#[test]
fn type_state_builder_sql_postgres() {
  let q = RelationalQuery::<(AuthorRow,)>::new("users", &["id", "name"]).with_many::<PostSummary>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = q.to_sql_postgres();
  assert!(
    sql.contains("LEFT JOIN LATERAL"),
    "Postgres SQL should use lateral join: {sql}"
  );
  assert!(
    sql.contains(r#"FROM "users""#),
    "should select from users: {sql}"
  );
}

#[test]
fn type_state_builder_sql_sqlite() {
  let q = RelationalQuery::<(AuthorRow,)>::new("users", &["id", "name"]).with_many::<PostSummary>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = q.to_sql_sqlite();
  assert!(
    sql.contains("json_group_array"),
    "SQLite SQL should use json_group_array: {sql}"
  );
  assert!(
    !sql.contains("LATERAL"),
    "SQLite should not use LATERAL: {sql}"
  );
}

#[test]
fn relational_select_builder_postgres_vs_sqlite_structure() {
  let builder = RelationalSelectBuilder::new("users", &["id", "name", "email"])
    .with_many(
      "posts",
      "posts",
      "id",
      "author_id",
      &["id", "title", "body"],
    )
    .with_one("profile", "profiles", "id", "user_id", &["id", "bio"]);

  let pg_sql = builder.to_sql_postgres();
  let sqlite_sql = builder.to_sql_sqlite();

  assert!(
    pg_sql.contains("LEFT JOIN LATERAL"),
    "Postgres should use LATERAL: {pg_sql}"
  );
  assert!(
    pg_sql.contains("json_agg"),
    "Postgres should use json_agg: {pg_sql}"
  );

  assert!(
    !sqlite_sql.contains("LATERAL"),
    "SQLite should not use LATERAL: {sqlite_sql}"
  );
  assert!(
    sqlite_sql.contains("json_group_array"),
    "SQLite should use json_group_array: {sqlite_sql}"
  );

  assert!(pg_sql.contains(r#"FROM "users""#));
  assert!(sqlite_sql.contains(r#"FROM "users""#));
}

#[test]
fn json_round_trip_many_relation() -> Result<(), Box<dyn std::error::Error>> {
  let json = r#"[["p1","Title 1"],["p2","Title 2"],["p3","Title 3"]]"#;
  let rows = parse_json_array_of_arrays(json)?;
  assert_eq!(rows.len(), 3);
  for (i, row) in rows.iter().enumerate() {
    let deser = RelationDeserializer::new(row);
    let id: String = deser.get(0)?;
    assert_eq!(id, format!("p{}", i + 1));
  }
  Ok(())
}
