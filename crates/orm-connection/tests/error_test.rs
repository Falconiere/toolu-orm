use toolu_orm_connection::DbError;
use toolu_orm_core::error::DbCoreError;

#[test]
fn connection_error_displays_message() {
  let err = DbError::Connection("refused".to_owned());
  assert_eq!(err.to_string(), "connection failed: refused");
}

#[test]
fn query_error_displays_message() {
  let err = DbError::Query("syntax error".to_owned());
  assert_eq!(err.to_string(), "query failed: syntax error");
}

#[test]
fn transaction_error_displays_message() {
  let err = DbError::Transaction("deadlock".to_owned());
  assert_eq!(err.to_string(), "transaction failed: deadlock");
}

#[test]
fn pool_error_displays_message() {
  let err = DbError::Pool("exhausted".to_owned());
  assert_eq!(err.to_string(), "pool error: exhausted");
}

#[test]
fn row_mapping_error_displays_message() {
  let err = DbError::RowMapping("column 3 type mismatch".to_owned());
  assert_eq!(
    err.to_string(),
    "row mapping failed: column 3 type mismatch"
  );
}

#[test]
fn from_db_core_error_produces_row_mapping() {
  let core_err = DbCoreError::RowMapping("bad column".to_owned());
  let err: DbError = core_err.into();
  assert_eq!(
    err.to_string(),
    "row mapping failed: row mapping error: bad column"
  );
}
