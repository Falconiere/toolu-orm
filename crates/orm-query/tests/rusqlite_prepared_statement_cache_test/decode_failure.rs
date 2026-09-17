//! A `FromRow` decode failure through a cached statement still surfaces as
//! `QueryError::RowMapping`, unchanged from the uncached path.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;
use toolu_orm_query::executor::Executor;
use toolu_orm_query::QueryError;

use crate::db;
use crate::users::insert_user;

/// Decodes column 0 as an integer regardless of its actual type, so selecting
/// a text column (`name`) through it forces a rusqlite type-mismatch error,
/// which `Executor::query_map` must surface as `QueryError::RowMapping`. No
/// decoded value is kept -- only whether decoding itself succeeds matters.
#[derive(Debug)]
struct MismatchedRow;

impl FromRow for MismatchedRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let _: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self)
  }
}

#[test]
fn query_map_decode_failure_is_row_mapping_error() -> Result<(), Box<dyn std::error::Error>> {
  let conn = db::setup_db()?;
  insert_user(&conn, "u1", "Alice", "alice@example.com", Some(30))?;

  // `name` is TEXT; decoding column 0 as i64 must fail.
  let result: Result<Vec<MismatchedRow>, QueryError> = Executor::query_map(
    &conn,
    "SELECT name FROM users WHERE id = ?1",
    vec![Value::Text("u1".to_owned())],
  );

  match result {
    Err(QueryError::RowMapping { .. }) => {},
    other => return Err(format!("expected RowMapping, got: {other:?}").into()),
  }
  Ok(())
}
