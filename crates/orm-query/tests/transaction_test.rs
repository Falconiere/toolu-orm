use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::row::FromRow;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::transaction::TransactionExt;
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn transaction_commit_on_ok() -> TestResult {
  let conn = setup_db().await?;

  conn
    .run_transaction(|tx| async move {
      InsertBuilder::new("test_users")
        .set(&ID, "tx1")
        .set(&NAME, "Transaction User")
        .set(&AGE, 30i64)
        .execute(&tx)
        .await?;
      Ok(())
    })
    .await?;

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_all(&conn)
    .await?;
  assert_eq!(users.len(), 1);
  assert_eq!(users.first().map(|u| u.id.as_str()), Some("tx1"));
  assert_eq!(
    users.first().map(|u| u.name.as_str()),
    Some("Transaction User")
  );
  assert_eq!(users.first().map(|u| u.age), Some(30));
  Ok(())
}

#[tokio::test]
async fn transaction_rollback_on_err() -> TestResult {
  let conn = setup_db().await?;

  let result: Result<(), QueryError> = conn
    .run_transaction(|tx| async move {
      InsertBuilder::new("test_users")
        .set(&ID, "tx2")
        .set(&NAME, "Should Not Exist")
        .set(&AGE, 25i64)
        .execute(&tx)
        .await?;
      Err(QueryError::NotFound {
        table: "test".to_owned(),
      })
    })
    .await;

  assert!(result.is_err());

  let users: Vec<TestUser> = SelectBuilder::new("test_users")
    .columns_raw(&["id", "name", "age"])
    .fetch_all(&conn)
    .await?;
  assert_eq!(users.len(), 0);
  Ok(())
}

#[tokio::test]
async fn transaction_multiple_operations() -> TestResult {
  let conn = setup_db().await?;

  conn
    .run_transaction(|tx| async move {
      InsertBuilder::new("test_users")
        .set(&ID, "tx3")
        .set(&NAME, "First")
        .set(&AGE, 20i64)
        .execute(&tx)
        .await?;
      InsertBuilder::new("test_users")
        .set(&ID, "tx4")
        .set(&NAME, "Second")
        .set(&AGE, 30i64)
        .execute(&tx)
        .await?;
      Ok(())
    })
    .await?;

  let count = SelectBuilder::new("test_users").count(&conn).await?;
  assert_eq!(count, 2);
  Ok(())
}
