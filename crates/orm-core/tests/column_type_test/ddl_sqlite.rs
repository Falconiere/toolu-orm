use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;

#[test]
fn sqlite_text_returns_text() {
  assert_eq!(ColumnType::Text.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_integer_returns_integer() {
  assert_eq!(ColumnType::Integer.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_real_returns_real() {
  assert_eq!(ColumnType::Real.as_ddl_sql(Dialect::Sqlite), "REAL");
}

#[test]
fn sqlite_blob_returns_blob() {
  assert_eq!(ColumnType::Blob.as_ddl_sql(Dialect::Sqlite), "BLOB");
}

#[test]
fn sqlite_uuid_returns_text() {
  assert_eq!(ColumnType::Uuid.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_boolean_returns_integer() {
  assert_eq!(ColumnType::Boolean.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_timestamp_returns_integer() {
  assert_eq!(ColumnType::Timestamp.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_date_returns_text() {
  assert_eq!(ColumnType::Date.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_time_returns_text() {
  assert_eq!(ColumnType::Time.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_json_returns_text() {
  assert_eq!(ColumnType::Json.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_jsonb_returns_text() {
  assert_eq!(ColumnType::Jsonb.as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_bigint_returns_integer() {
  assert_eq!(ColumnType::BigInt.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_smallint_returns_integer() {
  assert_eq!(ColumnType::SmallInt.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_serial_returns_integer() {
  assert_eq!(ColumnType::Serial.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_big_serial_returns_integer() {
  assert_eq!(ColumnType::BigSerial.as_ddl_sql(Dialect::Sqlite), "INTEGER");
}

#[test]
fn sqlite_numeric_returns_real() {
  assert_eq!(ColumnType::Numeric.as_ddl_sql(Dialect::Sqlite), "REAL");
}

#[test]
fn sqlite_varchar_returns_text() {
  assert_eq!(ColumnType::Varchar(255).as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_char_returns_text() {
  assert_eq!(ColumnType::Char(10).as_ddl_sql(Dialect::Sqlite), "TEXT");
}

#[test]
fn sqlite_array_returns_text() {
  let ct = ColumnType::Array(Box::new(ColumnType::Integer));
  assert_eq!(ct.as_ddl_sql(Dialect::Sqlite), "TEXT");
}
