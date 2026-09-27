use toolu_orm_connection::{DbConnection, LanceConnection, LanceDbConnection};
use toolu_orm_core::{
  column::{Blob, Boolean, Integer, Real, Text},
  error::DbCoreError,
  query_column::Column,
  row::{FromRow, LanceRow},
  value::Value,
};
use toolu_orm_query::insert::InsertBuilder;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const ID: Column<Integer> = Column::new("insert_source", "id");
pub const LABEL: Column<Text> = Column::new("insert_source", "label");
pub const SCORE: Column<Real> = Column::new("insert_source", "score");
pub const LIVE: Column<Boolean> = Column::new("insert_source", "live");
pub const BYTES: Column<Blob> = Column::new("insert_source", "bytes");
pub const NOTE: Column<Text> = Column::new("insert_source", "note");
pub const TEXT: &str = "O'Brien ?1 $2; DROP TABLE insert_source; -- 東京";
const COLUMNS: &[&str] = &["id", "label", "score", "live", "bytes", "note"];

#[derive(Debug, PartialEq)]
pub struct Stored(pub Vec<Value>);

impl FromRow for Stored {
  const REQUIRED_COLUMNS: &'static [&'static str] = COLUMNS;

  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    COLUMNS
      .iter()
      .map(|name| row.get(name).cloned())
      .collect::<Result<Vec<_>, _>>()
      .map(Self)
  }
}

/// Open the real Lance fixture, including previously persisted tables.
///
/// # Errors
/// Propagates missing extension configuration, startup and attach failures.
pub fn open(directory: &std::path::Path) -> Result<LanceDbConnection, Box<dyn std::error::Error>> {
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  Ok(LanceDbConnection::from_namespace(
    LanceConnection::open(extension)?.attach(directory, "data")?,
  ))
}

/// Read every column in deterministic ID order.
///
/// # Errors
/// Propagates query and scalar decoding failures.
pub async fn rows(conn: &LanceDbConnection) -> Result<Vec<Stored>, Box<dyn std::error::Error>> {
  Ok(
    conn
      .query_map(
        "SELECT id, label, score, live, bytes, note FROM insert_source ORDER BY id",
        vec![],
      )
      .await?,
  )
}

/// Create a real table and insert two distinguishable rows.
///
/// # Errors
/// Propagates table creation and insertion failures.
pub async fn seed(conn: &LanceDbConnection) -> TestResult {
  conn.execute_batch("CREATE TABLE insert_source (id BIGINT, label VARCHAR, score DOUBLE, live BOOLEAN, bytes BLOB, note VARCHAR)").await?;
  for id in [1_i64, 2] {
    InsertBuilder::new("insert_source")
      .set(&ID, id)
      .set(&LABEL, TEXT)
      .set(&SCORE, -12.5)
      .set(&LIVE, true)
      .set(&BYTES, vec![0_u8, 127, 255])
      .set(&NOTE, "original")
      .execute_on(conn)
      .await?;
  }
  Ok(())
}
