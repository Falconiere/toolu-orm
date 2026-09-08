//! `Value::vector`: the blob layout a `vec0` parameter is read from, and the
//! dimension check that belongs at the call site rather than at insert time.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::value::Value;

#[test]
fn an_embedding_becomes_its_little_endian_f32_bytes() {
  assert_eq!(
    Value::vector(&[1.0f32, -2.5]),
    Value::Blob(vec![0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x20, 0xc0])
  );
}

#[test]
fn the_blob_is_four_bytes_per_dimension() -> Result<(), Box<dyn std::error::Error>> {
  let embedding = vec![0.5f32; 768];
  let Value::Blob(bytes) = Value::vector(&embedding) else {
    return Err("expected a blob".into());
  };
  assert_eq!(bytes.len(), 768 * 4);
  assert_eq!(
    bytes
      .chunks_exact(4)
      .filter_map(|c| <[u8; 4]>::try_from(c).ok())
      .map(f32::from_le_bytes)
      .collect::<Vec<f32>>(),
    embedding
  );
  Ok(())
}

#[test]
fn an_empty_embedding_is_an_empty_blob() {
  assert_eq!(Value::vector(&[]), Value::Blob(vec![]));
}

#[test]
fn a_matching_length_passes_the_dimension_check() {
  assert_eq!(
    Value::vector_with_dim(&[1.0f32, 2.0], 2).ok(),
    Some(Value::vector(&[1.0f32, 2.0]))
  );
}

#[test]
fn a_mismatched_length_is_refused_with_both_numbers() {
  let error = Value::vector_with_dim(&[1.0f32, 2.0], 1024)
    .expect_err("expected the short embedding to be refused");
  assert!(
    matches!(
      error,
      DbCoreError::VectorDimension {
        expected: 1024,
        actual: 2
      }
    ),
    "wrong error: {error}"
  );
  assert_eq!(
    error.to_string(),
    "expected a 1024-element embedding, got 2"
  );
}
