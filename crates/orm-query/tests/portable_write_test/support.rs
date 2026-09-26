use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::{
  column::{Integer, Text},
  query_column::{Column, CommonOps},
};
use toolu_orm_query::{delete::DeleteBuilder, insert::InsertBuilder, update::UpdateBuilder};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
const ID: Column<Integer> = Column::new("portable_items", "id");
const LABEL: Column<Text> = Column::new("portable_items", "label");
const PAYLOAD: &str = "O'Brien ?1 $2; DROP TABLE portable_items; -- 東京";

async fn seed_rows(conn: &impl DbConnection) -> TestResult {
  for id in [7_i64, 8] {
    assert_eq!(
      InsertBuilder::new("portable_items")
        .set(&ID, id)
        .set(&LABEL, PAYLOAD)
        .execute_on(conn)
        .await?,
      1
    );
  }
  Ok(())
}

async fn update_rows(conn: &impl DbConnection) -> TestResult {
  assert_eq!(
    UpdateBuilder::new("portable_items")
      .set(&LABEL, "changed")
      .filter(ID.eq(99_i64))
      .execute_on(conn)
      .await?,
    0
  );
  assert_eq!(
    UpdateBuilder::new("portable_items")
      .set(&LABEL, "changed")
      .filter(LABEL.eq(PAYLOAD))
      .execute_on(conn)
      .await?,
    2
  );
  Ok(())
}

async fn delete_rows(conn: &impl DbConnection) -> TestResult {
  assert_eq!(
    DeleteBuilder::new("portable_items")
      .filter(ID.eq(99_i64))
      .execute_on(conn)
      .await?,
    0
  );
  assert_eq!(
    DeleteBuilder::new("portable_items")
      .filter(LABEL.eq("changed"))
      .execute_on(conn)
      .await?,
    2
  );
  assert_eq!(
    DeleteBuilder::new("portable_items")
      .execute_on(conn)
      .await?,
    0
  );
  Ok(())
}

async fn assert_missing_table_errors(conn: &impl DbConnection) -> TestResult {
  let errors = [
    InsertBuilder::new("missing_portable_items")
      .set(&ID, 9_i64)
      .execute_on(conn)
      .await,
    UpdateBuilder::new("missing_portable_items")
      .set(&ID, 9_i64)
      .execute_on(conn)
      .await,
    DeleteBuilder::new("missing_portable_items")
      .execute_on(conn)
      .await,
  ];
  for result in errors {
    assert!(matches!(result, Err(DbError::Query(message)) if !message.is_empty()));
  }
  Ok(())
}

async fn final_roundtrip(conn: &impl DbConnection) -> TestResult {
  assert_eq!(
    InsertBuilder::new("portable_items")
      .set(&ID, 10_i64)
      .set(&LABEL, PAYLOAD)
      .execute_on(conn)
      .await?,
    1
  );
  assert_eq!(
    DeleteBuilder::new("portable_items")
      .filter(ID.eq(10_i64))
      .filter(LABEL.eq(PAYLOAD))
      .execute_on(conn)
      .await?,
    1
  );
  Ok(())
}

// AC-1/2: exact stored values are checked by the subsequent bound filters.
// This routine has no dialect branch and uses the same builders on every driver.
pub async fn writes(conn: &impl DbConnection) -> TestResult {
  conn
    .execute_batch("CREATE TABLE portable_items (id BIGINT, label VARCHAR)")
    .await?;
  seed_rows(conn).await?;
  update_rows(conn).await?;
  delete_rows(conn).await?;
  assert_missing_table_errors(conn).await?;
  final_roundtrip(conn).await?;
  Ok(())
}
