use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::row::FromRow;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;
use toolu_orm_query::QueryError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── Test table columns ───────────────────────────────────────────────────────

const ID: Column<Text> = Column::new("test_users", "id");
const NAME: Column<Text> = Column::new("test_users", "name");
const AGE: Column<Integer> = Column::new("test_users", "age");

// ── TestUser ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct TestUser {
  id: String,
  name: String,
  age: i64,
}

impl FromRow for TestUser {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "name", "age"];

  fn from_pg_row(_: &tokio_postgres::Row) -> Result<Self, DbCoreError> {
    Err(DbCoreError::RowMapping(
      "TestUser is only decoded from libsql rows in this test".into(),
    ))
  }

  fn from_libsql_row(row: &libsql::Row) -> Result<Self, DbCoreError> {
    Ok(Self {
      id: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(format!("col 0: {e}")))?,
      name: row
        .get(1)
        .map_err(|e| DbCoreError::RowMapping(format!("col 1: {e}")))?,
      age: row
        .get(2)
        .map_err(|e| DbCoreError::RowMapping(format!("col 2: {e}")))?,
    })
  }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn setup_db() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn
    .execute(
      "CREATE TABLE test_users (id TEXT PRIMARY KEY, name TEXT NOT NULL, age INTEGER NOT NULL)",
      (),
    )
    .await?;
  Ok(conn)
}

async fn insert_user(
  conn: &libsql::Connection,
  id: &str,
  name: &str,
  age: i64,
) -> Result<(), QueryError> {
  InsertBuilder::new("test_users")
    .set(&ID, id)
    .set(&NAME, name)
    .set(&AGE, age)
    .execute(conn)
    .await?;
  Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn insert_and_select() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_all(&conn)
    .await?;

  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.id.as_str()), Some("u1"));
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Alice"));
  assert_eq!(users.first().map(|u| u.age), Some(30));
  Ok(())
}

#[tokio::test]
async fn select_empty_returns_empty_vec() -> TestResult {
  let conn = setup_db().await?;

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_all(&conn)
    .await?;

  assert!(users.is_empty());
  Ok(())
}

#[tokio::test]
async fn fetch_one_returns_single() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;

  let user: TestUser = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.id, "u1");
  assert_eq!(user.name, "Alice");
  assert_eq!(user.age, 30);
  Ok(())
}

#[tokio::test]
async fn fetch_one_empty_returns_not_found() -> TestResult {
  let conn = setup_db().await?;

  let result: Result<TestUser, QueryError> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_one(&conn)
    .await;

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(
    matches!(err, QueryError::NotFound { .. }),
    "expected NotFound, got: {err:?}"
  );
  Ok(())
}

#[tokio::test]
async fn fetch_optional_found() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;

  let user: Option<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_optional(&conn)
    .await?;

  assert!(user.is_some());
  assert_eq!(user.as_ref().map(|u| u.name.as_str()), Some("Alice"));
  Ok(())
}

#[tokio::test]
async fn fetch_optional_not_found() -> TestResult {
  let conn = setup_db().await?;

  let user: Option<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_optional(&conn)
    .await?;

  assert!(user.is_none());
  Ok(())
}

#[tokio::test]
async fn update_modifies_row() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;

  let affected = UpdateBuilder::new("test_users")
    .set(&NAME, "Bob")
    .filter(ID.eq("u1"))
    .execute(&conn)
    .await?;

  assert_eq!(affected, 1);

  let user: TestUser = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .filter(ID.eq("u1"))
    .fetch_one(&conn)
    .await?;

  assert_eq!(user.name, "Bob");
  Ok(())
}

#[tokio::test]
async fn delete_removes_row() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;

  let affected = DeleteBuilder::new("test_users")
    .filter(ID.eq("u1"))
    .execute(&conn)
    .await?;

  assert_eq!(affected, 1);

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_all(&conn)
    .await?;

  assert!(users.is_empty());
  Ok(())
}

#[tokio::test]
async fn count_returns_correct_number() -> TestResult {
  let conn = setup_db().await?;
  insert_user(&conn, "u1", "Alice", 30).await?;
  insert_user(&conn, "u2", "Bob", 25).await?;
  insert_user(&conn, "u3", "Charlie", 35).await?;

  let count = SelectBuilder::new("test_users").count(&conn).await?;

  assert_eq!(count, 3);

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .filter(NAME.eq("Alice"))
    .fetch_all(&conn)
    .await?;

  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.name.as_str()), Some("Alice"));
  Ok(())
}
