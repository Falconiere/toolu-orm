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
  // Building this test crate type-checks `write` with default (no-driver) features.
  use toolu_orm::connection::{require_capabilities, Capability};
  use toolu_orm::core::dialect::Dialect;
  assert!(matches!(
    require_capabilities(Dialect::Lance, &[Capability::OnConflict]),
    Err(DbError::UnsupportedCapability {
      capability: Capability::OnConflict,
      ..
    })
  ));
}

/// Compile the documented merge API using only facade imports.
/// # Errors
/// Propagates backend refusal, malformed input and database errors.
pub async fn merge_items(conn: &impl DbConnection) -> Result<u64, DbError> {
  use toolu_orm::{
    core::{column::Text, value::Value},
    query::merge::{Matched, MergeBuilder, NotMatched},
  };
  let id: Column<Integer> = Column::new("items", "id");
  let label: Column<Text> = Column::new("items", "label");
  MergeBuilder::new("items")
    .columns(&[&id, &label])
    .keys(&[&id])
    .row(vec![Value::Integer(1), Value::Text("updated".into())])
    .row(vec![Value::Integer(2), Value::Text("inserted".into())])
    .when_matched(Matched::Update)
    .when_not_matched(NotMatched::Insert)
    .execute_on(conn)
    .await
}
