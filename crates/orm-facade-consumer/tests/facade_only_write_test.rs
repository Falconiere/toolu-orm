//! Shared writes need only the facade, with any feature selection.

use toolu_orm::{
  connection::{DbConnection, DbError},
  core::{
    column::Integer,
    query_column::{Column, CommonOps},
  },
  query::{delete::DeleteBuilder, insert::InsertBuilder, update::UpdateBuilder},
};

/// Compile the same write function through facade paths.
/// # Errors
/// Propagates backend write failures.
pub async fn write(conn: &impl DbConnection) -> Result<u64, DbError> {
  let id: Column<Integer> = Column::new("items", "id");
  InsertBuilder::new("items")
    .set(&id, 7_i64)
    .execute_on(conn)
    .await?;
  UpdateBuilder::new("items")
    .set(&id, 8_i64)
    .filter(id.eq(7_i64))
    .execute_on(conn)
    .await?;
  DeleteBuilder::new("items")
    .filter(id.eq(8_i64))
    .execute_on(conn)
    .await
}

#[test]
fn shared_writes_compile_with_facade_only() {
  // Rust checks the generic public function body even with no driver enabled.
}
