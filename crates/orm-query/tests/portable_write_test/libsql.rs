use super::support::{writes, TestResult};
use toolu_orm_connection::{Database, DbConnection};
use toolu_orm_core::dialect::Dialect;

#[tokio::test]
async fn bound_writes_counts_and_errors() -> TestResult {
  let db = Database::init_local(":memory:").await?;
  let conn = db.connect()?;
  assert_eq!(conn.dialect(), Dialect::Sqlite);
  writes(&conn).await
}
