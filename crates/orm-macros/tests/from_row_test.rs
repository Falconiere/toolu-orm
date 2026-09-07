use toolu_orm_macros::FromRow;

#[derive(FromRow)]
pub struct User {
  pub id: String,
  pub name: String,
  pub age: i64,
}

#[test]
fn test_from_row_generates_impl_for_named_struct() {
  // This test verifies the derive macro compiles successfully.
  // The derive generates a FromRow impl — if this compiles, the macro works.
  // Actual DB integration (with libsql Row) is tested in integration tests.
  fn _assert_from_row<T: toolu_orm_core::row::FromRow>() {}
  _assert_from_row::<User>();
}

#[derive(FromRow)]
pub struct TestRequiredCols {
  pub id: String,
  pub name: String,
  pub age: i64,
}

#[test]
fn from_row_generates_required_columns() {
  assert_eq!(
    <TestRequiredCols as toolu_orm_core::row::FromRow>::REQUIRED_COLUMNS,
    &["id", "name", "age"]
  );
}
