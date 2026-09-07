//! Shared test table definition and setup for integration tests.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_macros::{table, FromRow};

// -- Test table ---------------------------------------------------------------

#[table(name = "integration_users")]
#[derive(FromRow, Debug, PartialEq)]
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
