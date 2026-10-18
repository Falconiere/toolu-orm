use super::support::{ELEMENTS, FINITE_BOUNDARIES, TestResult, invalid_inputs, value};
use toolu_orm_core::{value::Value, vector::Vector};

#[test]
fn sqlite_vec_round_trip_and_prewrite_validation() -> TestResult {
  toolu_orm_sqlite_vec_register::register()?;
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch("CREATE VIRTUAL TABLE vectors USING vec0(embedding float[3])")?;
  let insert = |elements: &[f32]| -> TestResult {
    conn.execute(
      "INSERT INTO vectors(embedding) VALUES (?1)",
      [value(elements)?],
    )?;
    Ok(())
  };
  insert(&ELEMENTS)?;
  for invalid in invalid_inputs() {
    assert!(insert(&invalid).is_err());
  }
  let (vector, storage, count): (Vector<3>, String, i64) = conn.query_row(
    "SELECT embedding, typeof(embedding), (SELECT count(*) FROM vectors) FROM vectors WHERE rowid = 1", [],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
  assert_eq!(vector.as_slice(), ELEMENTS);
  assert_eq!(storage, "blob");
  assert_eq!(count, 1);
  assert!(
    conn
      .query_row("SELECT embedding FROM vectors", [], |row| row
        .get::<_, Vector<2>>(0))
      .is_err()
  );
  let nullable: Option<Vector<3>> = conn.query_row("SELECT NULL", [], |row| row.get(0))?;
  assert!(nullable.is_none());
  assert!(
    conn
      .query_row("SELECT ?1", [Value::Blob(vec![0; 11])], |row| row
        .get::<_, Vector<3>>(0))
      .is_err()
  );
  assert!(
    conn
      .query_row("SELECT ?1", [Value::vector(&[1.0, f32::NAN, 3.0])], |row| {
        row.get::<_, Vector<3>>(0)
      })
      .is_err()
  );
  for elements in FINITE_BOUNDARIES {
    let vector: Vector<3> =
      conn.query_row("SELECT vec_f32(?1)", [value(&elements)?], |row| row.get(0))?;
    assert_eq!(vector.as_slice(), elements);
  }
  Ok(())
}
