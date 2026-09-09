//! Fetch/execute through `RusqliteConnection` as an `Executor`, with no runtime.

use toolu_orm_connection::RusqliteConnection;
use toolu_orm_core::column::Text;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::row::FromRow;
use toolu_orm_query::insert::InsertBuilder;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const USER_ID: Column<Text> = Column::new("users", "id");
const USER_NAME: Column<Text> = Column::new("users", "name");

#[derive(Debug)]
struct User {
  id: String,
  name: String,
}

impl FromRow for User {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "name"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self {
      id: row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
      name: row
        .get(1)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    })
  }
}

fn connect() -> Result<RusqliteConnection, Box<dyn std::error::Error>> {
  let raw = rusqlite::Connection::open_in_memory()?;
  raw.execute(
    "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
    (),
  )?;
  Ok(RusqliteConnection::from_connection(raw))
}

#[test]
fn insert_and_fetch_via_rusqlite_connection() -> TestResult {
  let conn = connect()?;

  InsertBuilder::new("users")
    .set(&USER_ID, "u1")
    .set(&USER_NAME, "Ann")
    .execute(&conn)?;

  let rows: Vec<User> = SelectBuilder::new("users")
    .columns_raw(&["id", "name"])
    .fetch_all(&conn)?;

  assert_eq!(rows.len(), 1);
  let row = rows.first().ok_or("expected one user")?;
  assert_eq!(row.id, "u1");
  assert_eq!(row.name, "Ann");
  Ok(())
}

#[test]
fn bad_sql_maps_to_query_error() -> TestResult {
  let conn = connect()?;
  let err: QueryError = SelectBuilder::new("nope")
    .columns_raw(&["id", "name"])
    .fetch_all::<User>(&conn)
    .expect_err("missing table");
  assert!(matches!(err, QueryError::Connection(_)));
  Ok(())
}
