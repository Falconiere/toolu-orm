//! Relational query execution (`with_many`/`with_one`) against a real
//! rusqlite connection (rusqlite-only lane, synchronous): correlated JSON
//! subqueries via `RelationalQuery::to_sql_sqlite()`, decoded with
//! `parse_many_column`/`parse_one_column` and
//! `FromRelationalRow::from_relational_values`.

#[path = "fixtures/rusqlite_db.rs"]
pub mod db;

use serde::Deserialize;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;
use toolu_orm_core::relational_row::FromRelationalRow;
use toolu_orm_macros::Relational;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::relational_builder::RelationalQuery;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── Columns & seeding ────────────────────────────────────────────────────────

const USER_ID: Column<Text> = Column::new("users", "id");
const USER_NAME: Column<Text> = Column::new("users", "name");
const USER_EMAIL: Column<Text> = Column::new("users", "email");
const USER_AGE: Column<Integer> = Column::new("users", "age");
const POST_ID: Column<Text> = Column::new("posts", "id");
const POST_AUTHOR_ID: Column<Text> = Column::new("posts", "author_id");
const POST_TITLE: Column<Text> = Column::new("posts", "title");

fn insert_user(conn: &rusqlite::Connection, id: &str, name: &str) -> TestResult {
  InsertBuilder::new("users")
    .set(&USER_ID, id)
    .set(&USER_NAME, name)
    .set(&USER_EMAIL, format!("{id}@example.com"))
    .set(&USER_AGE, 30_i64)
    .execute(conn)?;
  Ok(())
}

fn insert_post(
  conn: &rusqlite::Connection,
  id: &str,
  author_id: Option<&str>,
  title: &str,
) -> TestResult {
  let builder = InsertBuilder::new("posts")
    .set(&POST_ID, id)
    .set(&POST_TITLE, title);
  let builder = match author_id {
    Some(a) => builder.set(&POST_AUTHOR_ID, a),
    None => builder.set_null(&POST_AUTHOR_ID),
  };
  builder.execute(conn)?;
  Ok(())
}

/// u1 has 2 posts, u2 has 0 posts, p3 is an orphan post (author_id NULL).
fn seed(conn: &rusqlite::Connection) -> TestResult {
  insert_user(conn, "u1", "Alice")?;
  insert_user(conn, "u2", "Bob")?;
  insert_post(conn, "p1", Some("u1"), "First")?;
  insert_post(conn, "p2", Some("u1"), "Second")?;
  insert_post(conn, "p3", None, "Orphan")?;
  Ok(())
}

// ── Phantom row markers for `RelationalQuery<T>` ────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct UserRow {
  pub id: String,
  pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct PostRow {
  pub id: String,
  pub title: String,
}

// ── Real structs built via `FromRelationalRow` ──────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct PostSummary {
  pub id: String,
  pub title: String,
}

#[derive(Relational)]
#[relational(table = "users")]
struct UserWithPosts {
  pub id: String,
  pub name: String,
  #[has_many(table = "posts", foreign_key = "author_id", columns = ["id", "title"])]
  pub posts: Vec<PostSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuthorRow {
  pub id: String,
  pub name: String,
}

#[derive(Relational)]
#[relational(table = "posts")]
struct PostWithAuthor {
  pub id: String,
  pub title: String,
  #[belongs_to(table = "users", foreign_key = "author_id", columns = ["id", "name"])]
  pub author: Option<AuthorRow>,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn with_many_reports_correct_post_counts_via_parse_many_column() -> TestResult {
  let conn = db::setup_db()?;
  seed(&conn)?;

  let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"]).with_many::<PostRow>(
    "posts",
    "posts",
    "id",
    "author_id",
    &["id", "title"],
  );
  let sql = q.to_sql_sqlite();
  let mut stmt = conn.prepare(&sql)?;
  let mut rows = stmt.query([])?;

  let mut found_u1 = false;
  let mut found_u2 = false;
  while let Some(row) = rows.next()? {
    let id: String = row.get(0)?;
    let posts_json: String = row.get(2)?;
    let parsed = RelationalQuery::<(UserRow,)>::parse_many_column(&posts_json)?;
    match id.as_str() {
      "u1" => {
        assert_eq!(parsed.len(), 2);
        found_u1 = true;
      },
      "u2" => {
        assert!(parsed.is_empty());
        found_u2 = true;
      },
      other => return Err(format!("unexpected user id: {other}").into()),
    }
  }
  assert!(found_u1 && found_u2);
  Ok(())
}

#[test]
fn with_many_builds_structs_via_from_relational_values() -> TestResult {
  let conn = db::setup_db()?;
  seed(&conn)?;

  let q = RelationalQuery::<(UserRow,)>::new("users", UserWithPosts::SCALAR_COLUMNS)
    .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"]);
  let sql = q.to_sql_sqlite();
  let mut stmt = conn.prepare(&sql)?;
  let mut rows = stmt.query([])?;

  let mut found_u1 = false;
  let mut found_u2 = false;
  while let Some(row) = rows.next()? {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let posts_json: String = row.get(2)?;
    let posts_rows = RelationalQuery::<(UserRow,)>::parse_many_column(&posts_json)?;
    let posts_value = serde_json::Value::Array(
      posts_rows
        .into_iter()
        .map(serde_json::Value::Array)
        .collect(),
    );
    let values = vec![serde_json::json!(id), serde_json::json!(name), posts_value];
    let user = UserWithPosts::from_relational_values(&values)?;

    match id.as_str() {
      "u1" => {
        assert_eq!(user.id, id);
        assert_eq!(user.name, name);
        let mut titles: Vec<&str> = user.posts.iter().map(|p| p.title.as_str()).collect();
        titles.sort_unstable();
        assert_eq!(titles, vec!["First", "Second"]);
        let mut post_ids: Vec<&str> = user.posts.iter().map(|p| p.id.as_str()).collect();
        post_ids.sort_unstable();
        assert_eq!(post_ids, vec!["p1", "p2"]);
        found_u1 = true;
      },
      "u2" => {
        assert_eq!(user.name, name);
        assert!(user.posts.is_empty());
        found_u2 = true;
      },
      other => return Err(format!("unexpected user id: {other}").into()),
    }
  }
  assert!(found_u1 && found_u2);
  Ok(())
}

#[test]
fn with_one_returns_none_for_orphan_post_and_some_for_authored_post() -> TestResult {
  let conn = db::setup_db()?;
  seed(&conn)?;

  let q = RelationalQuery::<(PostRow,)>::new("posts", PostWithAuthor::SCALAR_COLUMNS)
    .with_one::<UserRow>("author", "users", "author_id", "id", &["id", "name"]);
  let sql = q.to_sql_sqlite();
  let mut stmt = conn.prepare(&sql)?;
  let mut rows = stmt.query([])?;

  let mut orphan_checked = false;
  let mut authored_checked = false;
  while let Some(row) = rows.next()? {
    let id: String = row.get(0)?;
    let title: String = row.get(1)?;
    let author_json: Option<String> = row.get(2)?;
    let author_value = match author_json {
      None => serde_json::Value::Null,
      Some(text) => {
        let author_rows = RelationalQuery::<(PostRow,)>::parse_one_column(&text)?;
        serde_json::Value::Array(author_rows)
      },
    };
    let values = vec![
      serde_json::json!(id),
      serde_json::json!(title),
      author_value,
    ];
    let post = PostWithAuthor::from_relational_values(&values)?;

    if id == "p3" {
      assert_eq!(post.id, id);
      assert_eq!(post.title, title);
      assert!(post.author.is_none());
      orphan_checked = true;
    } else if id == "p1" {
      assert_eq!(post.title, title);
      let author = post.author.as_ref().ok_or("expected author")?;
      assert_eq!(author.id, "u1");
      assert_eq!(author.name, "Alice");
      authored_checked = true;
    }
  }
  assert!(orphan_checked && authored_checked);
  Ok(())
}
