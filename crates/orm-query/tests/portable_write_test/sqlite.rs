use super::support::{writes, TestResult};
use toolu_orm_connection::{DbConnection, RusqliteConnection};
use toolu_orm_core::dialect::Dialect;

#[tokio::test]
async fn bound_writes_counts_and_errors() -> TestResult {
  #[cfg(feature = "sqlite-vec")]
  toolu_orm_sqlite_vec_register::register()?;
  let raw = rusqlite::Connection::open_in_memory()?;
  #[cfg(feature = "sqlite-vec")]
  {
    let version: String = raw.query_row("SELECT vec_version()", [], |row| row.get(0))?;
    assert_eq!(version, "v0.1.9");
  }
  let conn = RusqliteConnection::from_connection(raw);
  assert_eq!(conn.dialect(), Dialect::Sqlite);
  writes(&conn).await
}
