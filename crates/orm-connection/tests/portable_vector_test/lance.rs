use super::support::{ELEMENTS, FINITE_BOUNDARIES, TestResult, invalid_inputs, value};
use toolu_orm_connection::{
  DbConnectionBlocking, DbError, LanceConnection, LanceDbConnection, to_duckdb_params,
};
use toolu_orm_core::{
  error::DbCoreError,
  row::{FromRow, LanceRow},
  vector::Vector,
};

#[derive(Debug)]
struct Embedding(Option<Vector<3>>);
impl FromRow for Embedding {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["embedding"];
  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    Ok(Self(row.get_typed("embedding")?))
  }
}

#[derive(Debug)]
struct RequiredEmbedding;
impl FromRow for RequiredEmbedding {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["embedding"];
  fn from_lance_row(row: &LanceRow) -> Result<Self, DbCoreError> {
    let _: Vector<3> = row.get_typed("embedding")?;
    Ok(Self)
  }
}

#[test]
fn lance_array_round_trip_and_prewrite_validation() -> TestResult {
  let directory = tempfile::tempdir()?;
  let extension =
    std::env::var_os("LANCE_EXTENSION_PATH").ok_or("LANCE_EXTENSION_PATH required")?;
  let namespace = LanceConnection::open(extension)?.attach(directory.path(), "data")?;
  namespace
    .connection()
    .execute_batch("CREATE TABLE vectors (embedding FLOAT[3])")?;
  let insert = |elements: &[f32]| -> TestResult {
    let params = to_duckdb_params(&[value(elements)?])?;
    namespace.connection().execute(
      "INSERT INTO vectors VALUES (?1)",
      duckdb::params_from_iter(params.iter()),
    )?;
    Ok(())
  };
  insert(&ELEMENTS)?;
  for invalid in invalid_inputs() {
    assert!(insert(&invalid).is_err());
  }
  let (storage, count): (String, i64) = namespace.connection().query_row(
    "SELECT typeof(embedding), (SELECT count(*) FROM vectors) FROM vectors",
    [],
    |row| Ok((row.get(0)?, row.get(1)?)),
  )?;
  assert_eq!(storage, "FLOAT[3]");
  assert_eq!(count, 1);
  let session = LanceDbConnection::from_namespace(namespace);
  let rows: Vec<Embedding> = session.query_map("SELECT embedding FROM vectors", vec![])?;
  assert_eq!(
    rows
      .first()
      .ok_or("no row")?
      .0
      .as_ref()
      .ok_or("NULL")?
      .as_slice(),
    ELEMENTS
  );
  let rows: Vec<Embedding> = session.query_map("SELECT NULL::FLOAT[3] AS embedding", vec![])?;
  assert!(rows.first().ok_or("no NULL row")?.0.is_none());
  let required_null = session
    .query_map::<RequiredEmbedding>("SELECT NULL::FLOAT[3] AS embedding", vec![])
    .expect_err("required NULL");
  assert!(
    matches!(required_null, DbError::RowMapping(ref message) if message.contains("embedding"))
  );
  for sql in [
    "SELECT [1,2]::FLOAT[2] AS embedding",
    "SELECT [1,2,3]::INTEGER[3] AS embedding",
    "SELECT [1,2,3]::DOUBLE[3] AS embedding",
    "SELECT [1,2,3]::FLOAT[] AS embedding",
    "SELECT [{'a': 1}]::STRUCT(a INTEGER)[1] AS embedding",
    "SELECT [1,NULL,3]::FLOAT[3] AS embedding",
    "SELECT [1,'NaN'::FLOAT,3]::FLOAT[3] AS embedding",
  ] {
    let error = session
      .query_map::<Embedding>(sql, vec![])
      .expect_err("invalid vector");
    assert!(
      matches!(error, DbError::RowMapping(ref message) if message.contains("embedding")),
      "{error}"
    );
  }
  for elements in FINITE_BOUNDARIES {
    let rows: Vec<Embedding> = session.query_map(
      "SELECT CAST(?1 AS FLOAT[3]) AS embedding",
      vec![value(&elements)?],
    )?;
    assert_eq!(
      rows
        .first()
        .ok_or("no row")?
        .0
        .as_ref()
        .ok_or("NULL")?
        .as_slice(),
      elements
    );
  }
  Ok(())
}
