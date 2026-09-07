use toolu_orm_core::column::ColumnType;

#[test]
fn serial_variant_exists() {
  let ct = ColumnType::Serial;
  assert_eq!(ct, ColumnType::Serial);
}

#[test]
fn big_serial_variant_exists() {
  let ct = ColumnType::BigSerial;
  assert_eq!(ct, ColumnType::BigSerial);
}

#[test]
fn jsonb_variant_exists() {
  let ct = ColumnType::Jsonb;
  assert_eq!(ct, ColumnType::Jsonb);
}

#[test]
fn numeric_variant_exists() {
  let ct = ColumnType::Numeric;
  assert_eq!(ct, ColumnType::Numeric);
}

#[test]
fn char_variant_exists() {
  let ct = ColumnType::Char(10);
  assert_eq!(ct, ColumnType::Char(10));
}

#[test]
fn array_variant_exists() {
  let ct = ColumnType::Array(Box::new(ColumnType::Text));
  assert_eq!(ct, ColumnType::Array(Box::new(ColumnType::Text)));
}

#[test]
fn nested_array_variant() {
  let ct = ColumnType::Array(Box::new(ColumnType::Integer));
  assert_eq!(ct, ColumnType::Array(Box::new(ColumnType::Integer)));
}
