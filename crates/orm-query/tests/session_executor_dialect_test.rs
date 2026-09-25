#![cfg(not(feature = "postgres"))]

use std::cell::RefCell;

use rusqlite::Connection;
use toolu_orm_connection::RusqliteConnection;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::update::UpdateBuilder;
use toolu_orm_query::QueryError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: Column<Integer> = Column::new("items", "id");
const NAME: Column<Text> = Column::new("items", "name");

struct IdRow {
  id: i64,
}

impl FromRow for IdRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      id: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(format!("id: {e}")))?,
    })
  }
}

struct CaptureExecutor<'a> {
  connection: &'a Connection,
  sql: RefCell<Vec<String>>,
}

impl Executor for CaptureExecutor<'_> {
  fn dialect(&self) -> Dialect {
    Dialect::Postgres
  }

  fn execute_sql(&self, sql: &str, params: Vec<Value>) -> Result<u64, QueryError> {
    self.sql.borrow_mut().push(sql.to_owned());
    Executor::execute_sql(self.connection, sql, params)
  }

  fn query_map<T: FromRow>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>, QueryError> {
    self.sql.borrow_mut().push(sql.to_owned());
    Executor::query_map(self.connection, sql, params)
  }
}

#[test]
fn execution_uses_runtime_dialect_even_when_current_is_sqlite() -> TestResult {
  assert_eq!(Dialect::CURRENT, Dialect::Sqlite);
  let connection = Connection::open_in_memory()?;
  connection.execute_batch(
    "CREATE TABLE items (id INTEGER, name TEXT);\
     INSERT INTO items VALUES (1, 'one'), (2, 'two');",
  )?;
  let session = CaptureExecutor {
    connection: &connection,
    sql: RefCell::new(Vec::new()),
  };

  let rows: Vec<IdRow> = SelectBuilder::new("items")
    .columns_raw(&["id"])
    .filter(ID.eq(1_i64))
    .fetch_all(&session)?;
  assert_eq!(rows.into_iter().map(|r| r.id).collect::<Vec<_>>(), vec![1]);
  assert_eq!(
    SelectBuilder::new("items")
      .columns_raw(&["id"])
      .filter(ID.eq(2_i64))
      .fetch_one::<IdRow>(&session)?
      .id,
    2
  );
  assert!(SelectBuilder::new("items")
    .columns_raw(&["id"])
    .filter(ID.eq(99_i64))
    .fetch_optional::<IdRow>(&session)?
    .is_none());
  assert_eq!(
    SelectBuilder::new("items")
      .filter(ID.gt(0_i64))
      .count(&session)?,
    2
  );
  assert!(!SelectBuilder::new("items")
    .filter(ID.eq(99_i64))
    .exists(&session)?);

  assert_eq!(
    InsertBuilder::new("items")
      .set(&ID, 3_i64)
      .set(&NAME, "three")
      .execute(&session)?,
    1
  );
  assert_eq!(
    UpdateBuilder::new("items")
      .set(&NAME, "THREE")
      .filter(ID.eq(3_i64))
      .execute(&session)?,
    1
  );
  assert_eq!(
    DeleteBuilder::new("items")
      .filter(ID.eq(1_i64))
      .execute(&session)?,
    1
  );
  assert_eq!(
    connection.query_row("SELECT name FROM items WHERE id = 3", [], |row| row
      .get::<_, String>(0))?,
    "THREE"
  );
  let sent = session.sql.borrow();
  assert_eq!(
    sent.as_slice(),
    [
      "SELECT \"id\" FROM \"items\" WHERE \"items\".\"id\" = $1",
      "SELECT \"id\" FROM \"items\" WHERE \"items\".\"id\" = $1 LIMIT $2",
      "SELECT \"id\" FROM \"items\" WHERE \"items\".\"id\" = $1 LIMIT $2",
      "SELECT COUNT(*) FROM \"items\" WHERE \"items\".\"id\" > $1",
      "SELECT EXISTS(SELECT 1 FROM \"items\" WHERE \"items\".\"id\" = $1)",
      "INSERT INTO \"items\" (\"id\", \"name\") VALUES ($1, $2)",
      "UPDATE \"items\" SET \"name\" = $1 WHERE \"items\".\"id\" = $2",
      "DELETE FROM \"items\" WHERE \"items\".\"id\" = $1",
    ]
  );
  Ok(())
}

#[test]
fn built_in_sqlite_executors_report_sqlite() -> TestResult {
  let raw = Connection::open_in_memory()?;
  assert_eq!(Executor::dialect(&raw), Dialect::Sqlite);
  let wrapped = RusqliteConnection::from_connection(raw);
  assert_eq!(Executor::dialect(&wrapped), Dialect::Sqlite);
  Ok(())
}
