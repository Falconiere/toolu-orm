//! Shared test table definition and setup for integration tests.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_macros::table;

// -- Test table ---------------------------------------------------------------

#[table(name = "integration_users")]
#[derive(Debug, PartialEq)]
pub struct IntegrationUser {
  #[column(primary_key, column_type = "Text")]
  pub id: String,
  #[column(not_null, column_type = "Text")]
  pub name: String,
  #[column(not_null, column_type = "Text")]
  pub email: String,
  #[column(column_type = "Integer")]
  pub age: i64,
}

// `#[derive(FromRow)]` only decodes Postgres rows (its libsql method is a
// stub), so the libsql-only lane maps rows by hand.
impl FromRow for IntegrationUser {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "name", "email", "age"];

  fn from_row(row: &libsql::Row) -> Result<Self, DbCoreError> {
    let col = |i: i32, e: libsql::Error| DbCoreError::RowMapping(format!("column {i}: {e}"));
    Ok(Self {
      id: row.get(0).map_err(|e| col(0, e))?,
      name: row.get(1).map_err(|e| col(1, e))?,
      email: row.get(2).map_err(|e| col(2, e))?,
      age: row.get(3).map_err(|e| col(3, e))?,
    })
  }
}

// -- Setup --------------------------------------------------------------------

pub(crate) async fn setup() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn
    .execute(
      "CREATE TABLE integration_users (id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, age INTEGER)",
      (),
    )
    .await?;
  Ok(conn)
}
