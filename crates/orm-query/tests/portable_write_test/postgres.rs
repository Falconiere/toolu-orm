use super::support::{writes, TestResult};
use toolu_orm_connection::{DbConnection, PgConfig, PgDatabase};
use toolu_orm_core::dialect::Dialect;

#[tokio::test]
async fn bound_writes_counts_and_errors() -> TestResult {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let mut conn = db.connect().await?;
  assert_eq!(conn.dialect(), Dialect::Postgres);
  let schema = format!("portable_writes_173_{}", std::process::id());
  conn
    .execute_batch(&format!(
      "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; SET search_path TO {schema}"
    ))
    .await?;
  let result = writes(&conn).await;
  let txn = conn.transaction().await?;
  assert_eq!(txn.dialect(), Dialect::Postgres);
  txn.commit().await?;
  conn
    .execute_batch(&format!("DROP SCHEMA {schema} CASCADE"))
    .await?;
  result
}
