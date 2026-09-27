use toolu_orm_connection::{DbConnection, LanceConnection, LanceDbConnection, LanceNamespace};
use toolu_orm_core::{
  column::{Blob, Boolean, Integer, Real, Text},
  query_column::Column,
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

#[derive(Debug, PartialEq)]
pub struct Stored(pub Vec<Value>);

fn namespace(directory: &std::path::Path) -> Result<LanceNamespace, Box<dyn std::error::Error>> {
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  Ok(LanceConnection::open(extension)?.attach(directory, "data")?)
}

/// Open the real Lance fixture, including previously persisted tables.
///
/// # Errors
/// Propagates missing extension configuration, startup and attach failures.
pub fn open(directory: &std::path::Path) -> Result<LanceDbConnection, Box<dyn std::error::Error>> {
  Ok(LanceDbConnection::from_namespace(namespace(directory)?))
}

/// Read every persisted column through an independent connection in ID order.
///
/// # Errors
/// Propagates query and scalar decoding failures.
pub fn rows(directory: &std::path::Path) -> Result<Vec<Stored>, Box<dyn std::error::Error>> {
  let namespace = namespace(directory)?;
  let mut statement = namespace
    .connection()
    .prepare("SELECT id, label, score, live, bytes, note FROM insert_source ORDER BY id")?;
  let rows = statement.query_map([], |row| {
    Ok(Stored(vec![
      Value::Integer(row.get(0)?),
      Value::Text(row.get(1)?),
      Value::Real(row.get(2)?),
      Value::Boolean(row.get(3)?),
      Value::Blob(row.get(4)?),
      row
        .get::<_, Option<String>>(5)?
        .map_or(Value::Null, Value::Text),
    ]))
  })?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
