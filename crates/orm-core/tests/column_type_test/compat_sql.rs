use toolu_orm_core::column::ColumnType;

#[test]
fn as_sql_returns_turso_strict_names() {
  assert_eq!(ColumnType::Uuid.as_sql(), "uuid");
  assert_eq!(ColumnType::Boolean.as_sql(), "boolean");
  assert_eq!(ColumnType::Timestamp.as_sql(), "timestamp");
}

#[test]
fn as_compat_sql_returns_sqlite_compat_types() {
  assert_eq!(ColumnType::Uuid.as_compat_sql(), "TEXT");
  assert_eq!(ColumnType::Boolean.as_compat_sql(), "INTEGER");
  assert_eq!(ColumnType::Serial.as_compat_sql(), "INTEGER");
  assert_eq!(ColumnType::Jsonb.as_compat_sql(), "TEXT");
  assert_eq!(
    ColumnType::Array(Box::new(ColumnType::Text)).as_compat_sql(),
    "TEXT"
  );
}
