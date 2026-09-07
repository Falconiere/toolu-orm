use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;

#[test]
fn postgres_text_returns_text() {
  assert_eq!(ColumnType::Text.as_ddl_sql(Dialect::Postgres), "TEXT");
}

#[test]
fn postgres_integer_returns_integer() {
  assert_eq!(ColumnType::Integer.as_ddl_sql(Dialect::Postgres), "INTEGER");
}

#[test]
fn postgres_real_returns_double_precision() {
  assert_eq!(
    ColumnType::Real.as_ddl_sql(Dialect::Postgres),
    "DOUBLE PRECISION"
  );
}

#[test]
fn postgres_blob_returns_bytea() {
  assert_eq!(ColumnType::Blob.as_ddl_sql(Dialect::Postgres), "BYTEA");
}

#[test]
fn postgres_uuid_returns_uuid() {
  assert_eq!(ColumnType::Uuid.as_ddl_sql(Dialect::Postgres), "UUID");
}

#[test]
fn postgres_boolean_returns_boolean() {
  assert_eq!(ColumnType::Boolean.as_ddl_sql(Dialect::Postgres), "BOOLEAN");
}

#[test]
fn postgres_timestamp_returns_timestamptz() {
  assert_eq!(
    ColumnType::Timestamp.as_ddl_sql(Dialect::Postgres),
    "TIMESTAMPTZ"
  );
}

#[test]
fn postgres_date_returns_date() {
  assert_eq!(ColumnType::Date.as_ddl_sql(Dialect::Postgres), "DATE");
}

#[test]
fn postgres_time_returns_time() {
  assert_eq!(ColumnType::Time.as_ddl_sql(Dialect::Postgres), "TIME");
}

#[test]
fn postgres_json_returns_json() {
  assert_eq!(ColumnType::Json.as_ddl_sql(Dialect::Postgres), "JSON");
}

#[test]
fn postgres_jsonb_returns_jsonb() {
  assert_eq!(ColumnType::Jsonb.as_ddl_sql(Dialect::Postgres), "JSONB");
}

#[test]
fn postgres_bigint_returns_bigint() {
  assert_eq!(ColumnType::BigInt.as_ddl_sql(Dialect::Postgres), "BIGINT");
}

#[test]
fn postgres_smallint_returns_smallint() {
  assert_eq!(
    ColumnType::SmallInt.as_ddl_sql(Dialect::Postgres),
    "SMALLINT"
  );
}

#[test]
fn postgres_serial_returns_serial() {
  assert_eq!(ColumnType::Serial.as_ddl_sql(Dialect::Postgres), "SERIAL");
}

#[test]
fn postgres_big_serial_returns_bigserial() {
  assert_eq!(
    ColumnType::BigSerial.as_ddl_sql(Dialect::Postgres),
    "BIGSERIAL"
  );
}

#[test]
fn postgres_numeric_returns_numeric() {
  assert_eq!(ColumnType::Numeric.as_ddl_sql(Dialect::Postgres), "NUMERIC");
}

#[test]
fn postgres_varchar_returns_varchar_n() {
  assert_eq!(
    ColumnType::Varchar(255).as_ddl_sql(Dialect::Postgres),
    "VARCHAR(255)"
  );
}

#[test]
fn postgres_char_returns_char_n() {
  assert_eq!(
    ColumnType::Char(10).as_ddl_sql(Dialect::Postgres),
    "CHAR(10)"
  );
}

#[test]
fn postgres_array_returns_type_brackets() {
  let ct = ColumnType::Array(Box::new(ColumnType::Integer));
  assert_eq!(ct.as_ddl_sql(Dialect::Postgres), "INTEGER[]");
}

#[test]
fn postgres_array_of_uuid_returns_uuid_brackets() {
  let ct = ColumnType::Array(Box::new(ColumnType::Uuid));
  assert_eq!(ct.as_ddl_sql(Dialect::Postgres), "UUID[]");
}

#[test]
fn postgres_array_of_text_returns_text_brackets() {
  let ct = ColumnType::Array(Box::new(ColumnType::Text));
  assert_eq!(ct.as_ddl_sql(Dialect::Postgres), "TEXT[]");
}
