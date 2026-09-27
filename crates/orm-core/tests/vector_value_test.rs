//! Checked portable f32 vector construction and byte decoding.
use toolu_orm_core::{
  error::DbCoreError,
  value::Value,
  vector::{Vector, VectorValue},
};

#[test]
fn checked_values_keep_dimensions_and_finite_elements() -> Result<(), DbCoreError> {
  for elements in [
    [1.25, -2.5, 0.0],
    [0.0; 3],
    [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1)],
  ] {
    let vector = Vector::<3>::new(&elements)?;
    assert_eq!(vector.as_slice(), elements);
    let Value::Vector(value) = Value::from(vector) else {
      return Err(DbCoreError::RowMapping("lost vector tag".into()));
    };
    assert_eq!(value.as_slice(), elements);
    let bytes = value.sqlite_bytes();
    assert_eq!(Vector::<3>::from_sqlite_bytes(&bytes)?.as_slice(), elements);
  }
  Ok(())
}

#[test]
fn invalid_inputs_fail_before_value_construction() {
  for elements in [vec![], vec![1.0, 2.0], vec![1.0; 4]] {
    assert!(matches!(
      Vector::<3>::new(&elements),
      Err(DbCoreError::VectorDimension { expected: 3, .. })
    ));
  }
  assert!(Vector::<0>::new(&[]).is_err());
  for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
    assert!(Vector::<3>::new(&[1.0, invalid, 3.0]).is_err());
    assert!(VectorValue::new(&[invalid], 1).is_err());
  }
}

#[test]
fn sqlite_decode_rejects_bad_lengths_and_nonfinite_bytes() {
  for bytes in [
    vec![],
    vec![0; 11],
    vec![0; 16],
    f32::NAN.to_le_bytes().repeat(3),
  ] {
    assert!(Vector::<3>::from_sqlite_bytes(&bytes).is_err());
  }
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_decode_checks_binary_header_and_elements() {
  use postgres_types::{FromSql, Kind, Type};
  let ty = Type::new("vector".into(), 9999, Kind::Simple, "public".into());
  let mut bytes = vec![0, 3, 0, 0];
  for element in [1.25_f32, -2.5, 0.0] {
    bytes.extend_from_slice(&element.to_be_bytes());
  }
  assert_eq!(
    Vector::<3>::from_sql(&ty, &bytes).unwrap().as_slice(),
    [1.25, -2.5, 0.0]
  );
  assert!(Vector::<2>::from_sql(&ty, &bytes).is_err());
  assert!(Vector::<3>::from_sql(&ty, &[0, 3, 0, 1]).is_err());
  assert!(Vector::<3>::from_sql(&ty, &[0, 3]).is_err());
  let mut nonfinite = vec![0, 3, 0, 0];
  nonfinite.extend(f32::NAN.to_be_bytes().repeat(3));
  assert!(Vector::<3>::from_sql(&ty, &nonfinite).is_err());
}
