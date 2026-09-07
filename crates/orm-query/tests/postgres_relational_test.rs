//! `RelationalQuery` executed end-to-end on a live Postgres: the generated
//! `LEFT JOIN LATERAL` + `json_agg` SQL runs, and the JSON columns decode into
//! `#[derive(Relational)]` structs for `with_many` and `with_one`.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally; compiles only via the five-crate postgres lane.

#[path = "fixtures/postgres_db.rs"]
pub mod pg;

use std::collections::BTreeMap;

use toolu_orm_core::relational_row::FromRelationalRow;
use toolu_orm_macros::Relational;
use toolu_orm_query::relational_builder::RelationalQuery;

use pg::{client, insert_user, TestResult};

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct PostRow {
  id: String,
  title: String,
}

#[derive(Relational, Debug)]
#[relational(table = "users")]
struct UserWithPosts {
  id: String,
  name: String,
  #[has_many(table = "posts", foreign_key = "author_id", columns = ["id", "title"])]
  posts: Vec<PostRow>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct AuthorRow {
  id: String,
  name: String,
}

#[derive(Relational, Debug)]
#[relational(table = "posts")]
struct PostWithAuthor {
  id: String,
  title: String,
  #[belongs_to(table = "users", foreign_key = "author_id", columns = ["id", "name"])]
  author: Option<AuthorRow>,
}

/// u1 Ann with posts p1, p2 · u2 Bea with none · p3 with a NULL author.
async fn seeded(schema: &str) -> Result<tokio_postgres::Client, Box<dyn std::error::Error>> {
  let client = client(schema).await?;
  insert_user(&client, "u1", "Ann", "a@x.io", None).await?;
  insert_user(&client, "u2", "Bea", "b@x.io", None).await?;
  client
    .batch_execute(
      "INSERT INTO posts (id, author_id, title) VALUES \
       ('p1', 'u1', 'Hello'), ('p2', 'u1', 'World'), ('p3', NULL, 'Orphan')",
    )
    .await?;
  Ok(client)
}

/// Scalar columns come back as text, relation columns as JSON; both are handed
/// to the derive as `serde_json::Value`s in select order.
fn row_values(
  row: &tokio_postgres::Row,
  scalars: usize,
) -> Result<Vec<serde_json::Value>, tokio_postgres::Error> {
  let mut values = Vec::with_capacity(row.len());
  for i in 0..row.len() {
    if i < scalars {
      values.push(serde_json::Value::String(row.try_get::<usize, String>(i)?));
    } else {
      let json: Option<serde_json::Value> = row.try_get(i)?;
      values.push(json.unwrap_or(serde_json::Value::Null));
    }
  }
  Ok(values)
}

#[tokio::test]
async fn with_many_loads_children_as_one_statement() -> TestResult {
  let client = seeded("q_pg_rel_many").await?;
  let sql = RelationalQuery::<(UserWithPosts,)>::new("users", UserWithPosts::SCALAR_COLUMNS)
    .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"])
    .to_sql_postgres();
  assert!(sql.contains("LEFT JOIN LATERAL"), "sql: {sql}");

  let mut by_id = BTreeMap::new();
  for row in client.query(&sql, &[]).await? {
    let user = UserWithPosts::from_relational_values(&row_values(&row, 2)?)?;
    by_id.insert(user.id.clone(), user);
  }
  assert_eq!(by_id.len(), 2);

  let ann = by_id.get("u1").ok_or("u1 missing")?;
  assert_eq!(ann.name, "Ann");
  let mut titles: Vec<&str> = ann.posts.iter().map(|p| p.title.as_str()).collect();
  titles.sort_unstable();
  assert_eq!(titles, ["Hello", "World"]);

  let bea = by_id.get("u2").ok_or("u2 missing")?;
  assert!(bea.posts.is_empty(), "Bea has no posts: {:?}", bea.posts);
  Ok(())
}

#[tokio::test]
async fn with_one_loads_parent_and_null_parent() -> TestResult {
  let client = seeded("q_pg_rel_one").await?;
  let sql = RelationalQuery::<(PostWithAuthor,)>::new("posts", PostWithAuthor::SCALAR_COLUMNS)
    .with_one::<AuthorRow>("author", "users", "author_id", "id", &["id", "name"])
    .to_sql_postgres();

  let mut by_id = BTreeMap::new();
  for row in client.query(&sql, &[]).await? {
    let post = PostWithAuthor::from_relational_values(&row_values(&row, 2)?)?;
    by_id.insert(post.id.clone(), post);
  }
  assert_eq!(by_id.len(), 3);
  assert_eq!(by_id.get("p1").map(|p| p.title.as_str()), Some("Hello"));
  assert_eq!(
    by_id.get("p1").and_then(|p| p.author.clone()),
    Some(AuthorRow {
      id: "u1".into(),
      name: "Ann".into()
    })
  );
  assert_eq!(by_id.get("p3").map(|p| p.author.is_none()), Some(true));
  Ok(())
}
