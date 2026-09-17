//! A `FromRow` decode failure through a cached statement still surfaces as
//! `DbError::RowMapping`, unchanged from the uncached path.

use toolu_orm_connection::{DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

use crate::support::{SELECT_BY_ID, create_items, insert_item};

/// Decodes column 0 as an integer regardless of its actual type, so selecting
/// a text column (`label`) through it forces a rusqlite type-mismatch error,
/// which `query_map` must surface as `DbError::RowMapping`. No decoded value
/// is kept -- only whether decoding itself succeeds matters.
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
  let conn = crate::blocking_conn::open_in_memory()?;
  create_items(&conn)?;
  insert_item(&conn, 1, "alpha")?;

  // `label` is TEXT; decoding column 0 as i64 must fail.
  let result: Result<Vec<MismatchedRow>, DbError> =
    DbConnectionBlocking::query_map(&conn, SELECT_BY_ID, vec![Value::Integer(1)]);

  match result {
    Err(DbError::RowMapping(_)) => {},
    other => return Err(format!("expected DbError::RowMapping, got: {other:?}").into()),
  }
  Ok(())
}
