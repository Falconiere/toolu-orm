use super::merge::TestResult;
use toolu_orm_connection::{DbConnection, LanceConnection, LanceDbConnection};
pub type Rows = Vec<(i64, Option<String>, i64)>;

/// Real pinned Lance fixture operation.
/// # Errors
/// Propagates extension, database and decoding failures.
pub fn open(path: &std::path::Path) -> Result<LanceDbConnection, Box<dyn std::error::Error>> {
  let ext = std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  Ok(LanceDbConnection::from_namespace(
    LanceConnection::open(ext)?.attach(path, "data")?,
  ))
}

/// Real pinned Lance fixture operation.
/// # Errors
/// Propagates extension, database and decoding failures.
pub fn snapshot(path: &std::path::Path) -> Result<Rows, Box<dyn std::error::Error>> {
  let ext = std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let ns = LanceConnection::open(ext)?.attach(path, "data")?;
  let mut stmt = ns
    .connection()
    .prepare("SELECT id,label,score FROM items ORDER BY id,label,score")?;
  Ok(
    stmt
      .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
      .collect::<Result<Vec<_>, _>>()?,
  )
}

/// Real pinned Lance fixture operation.
/// # Errors
/// Propagates extension, database and decoding failures.
pub async fn seed(conn: &LanceDbConnection) -> TestResult {
  conn.execute_batch("CREATE TABLE items(id BIGINT,label VARCHAR,score BIGINT); INSERT INTO items VALUES(1,'old',10),(3,'untouched',30)").await?;
  Ok(())
}
