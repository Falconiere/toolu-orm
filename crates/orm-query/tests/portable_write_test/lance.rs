use super::support::{writes, TestResult};
use toolu_orm_connection::{DbConnection, LanceConnection, LanceDbConnection};
use toolu_orm_core::dialect::Dialect;

#[tokio::test]
async fn bound_writes_counts_and_errors() -> TestResult {
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let directory = tempfile::tempdir()?;
  let conn = LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(directory.path(), "data")?,
  );
  assert_eq!(conn.dialect(), Dialect::Lance);
  writes(&conn).await
}
