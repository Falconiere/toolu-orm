//! `users` row type, typed columns, and insert helper — shared by the libsql
//! mutations and reads scenario binaries. `#[derive(FromRow)]` expands to this
//! lane's single-backend shape, `from_row(&libsql::Row)` (see
//! `crates/orm-core/src/row/traits.rs`), so every suite below decodes through
//! the derive rather than a hand-written impl.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;
use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;

pub(crate) const USER_ID: Column<Text> = Column::new("users", "id");
pub(crate) const USER_NAME: Column<Text> = Column::new("users", "name");
pub(crate) const USER_EMAIL: Column<Text> = Column::new("users", "email");
pub(crate) const USER_AGE: Column<Integer> = Column::new("users", "age");
pub(crate) const USER_COLUMNS: [&str; 4] = ["id", "name", "email", "age"];

#[derive(FromRow, Debug, Clone, PartialEq)]
pub(crate) struct User {
  pub id: String,
  pub name: String,
  pub email: String,
  pub age: Option<i64>,
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
