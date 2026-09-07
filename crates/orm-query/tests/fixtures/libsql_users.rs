//! `users` row type, typed columns, and insert helper — shared by the libsql
//! mutations and reads scenario binaries (this lane's single-backend `FromRow`
//! shape, see `crates/orm-core/src/row/traits.rs`; `#[derive(FromRow)]` only
//! emits a working method for the postgres+libsql shape).

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::Column;
use toolu_orm_core::row::FromRow;
use toolu_orm_query::insert::InsertBuilder;

pub(crate) const USER_ID: Column<Text> = Column::new("users", "id");
pub(crate) const USER_NAME: Column<Text> = Column::new("users", "name");
pub(crate) const USER_EMAIL: Column<Text> = Column::new("users", "email");
pub(crate) const USER_AGE: Column<Integer> = Column::new("users", "age");
pub(crate) const USER_COLUMNS: [&str; 4] = ["id", "name", "email", "age"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct User {
  pub id: String,
  pub name: String,
  pub email: String,
  pub age: Option<i64>,
}

impl FromRow for User {
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

pub(crate) async fn insert_user(
  conn: &libsql::Connection,
  id: &str,
  name: &str,
  email: &str,
  age: Option<i64>,
) -> Result<(), Box<dyn std::error::Error>> {
  let builder = InsertBuilder::new("users")
    .set(&USER_ID, id)
    .set(&USER_NAME, name)
    .set(&USER_EMAIL, email);
  let builder = match age {
    Some(a) => builder.set(&USER_AGE, a),
    None => builder.set_null(&USER_AGE),
  };
  builder.execute(conn).await?;
  Ok(())
}
