//! A Lance session compiles with `toolu-orm` as the consumer's only dependency.

use toolu_orm::{
  connection::{DbConnection, DbError, LanceDbConnection},
  core::{
    error::DbCoreError,
    row::{FromRow, LanceRow},
    value::Value,
  },
};

struct ConsumerRow {
  id: i64,
}

impl FromRow for ConsumerRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id"];

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    let value = row.get("id")?;
    if let Value::Integer(id) = value {
      Ok(Self { id: *id })
    } else {
      Err(DbCoreError::RowMapping(format!(
        "id expected BIGINT, got {value:?}"
      )))
    }
  }
}

async fn use_all_connection_methods(conn: &LanceDbConnection) -> Result<Vec<ConsumerRow>, DbError> {
  DbConnection::execute_batch(conn, "CREATE TABLE items (id BIGINT)").await?;
  DbConnection::execute_sql(
    conn,
    "INSERT INTO items VALUES (?1)",
    vec![Value::Integer(7)],
  )
  .await?;
  DbConnection::query_map(
    conn,
    "SELECT id FROM items WHERE id = ?1",
    vec![Value::Integer(7)],
  )
  .await
}

#[test]
fn facade_only_lance_connection_methods_compile() {
  fn assert_send_sync<T: Send + Sync>() {}
  assert_send_sync::<LanceDbConnection>();
  let _ = use_all_connection_methods;
  let row = ConsumerRow { id: 7 };
  assert_eq!(row.id, 7);
}
