use toolu_orm_core::column::ColumnType;

#[test]
fn serial_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Serial;
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn big_serial_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::BigSerial;
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn jsonb_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Jsonb;
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn numeric_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Numeric;
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn char_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Char(50);
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn array_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Array(Box::new(ColumnType::Uuid));
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}

#[test]
fn array_of_varchar_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
  let ct = ColumnType::Array(Box::new(ColumnType::Varchar(255)));
  let json = serde_json::to_string(&ct)?;
  let back: ColumnType = serde_json::from_str(&json)?;
  assert_eq!(back, ct);
  Ok(())
}
