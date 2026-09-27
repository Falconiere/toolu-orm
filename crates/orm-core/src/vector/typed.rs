//! A vector whose declared dimension is part of its Rust type.
use super::{value::invalid, VectorValue};
use crate::{error::DbCoreError, value::Value};

/// A nonempty, finite f32 vector of exactly `N` elements.
///
/// ```
/// use toolu_orm_core::{vector::Vector, value::Value};
/// let embedding = Vector::<3>::new(&[1.25, -2.5, 0.0])?;
/// let parameter: Value = embedding.into();
/// # Ok::<(), toolu_orm_core::error::DbCoreError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Vector<const N: usize>(VectorValue);

impl<const N: usize> Vector<N> {
  /// Check the declaration before producing a database parameter.
  ///
  /// # Errors
  /// Rejects zero or unrepresentable dimensions, wrong length and nonfinite elements.
  pub fn new(elements: &[f32]) -> Result<Self, DbCoreError> {
    let dim =
      u32::try_from(N).map_err(|error| invalid(&format!("dimension exceeds u32: {error}")))?;
    VectorValue::new(elements, dim).map(Self)
  }

  /// Borrow the validated elements.
  #[must_use]
  pub fn as_slice(&self) -> &[f32] {
    self.0.as_slice()
  }

  /// Decode sqlite-vec's little-endian bytes and validate this declaration.
  ///
  /// # Errors
  /// Rejects incomplete f32 bytes, wrong dimensions, and nonfinite elements.
  pub fn from_sqlite_bytes(bytes: &[u8]) -> Result<Self, DbCoreError> {
    let mut elements = Vec::with_capacity(bytes.len() / 4);
    let chunks = bytes.chunks_exact(4);
    if !chunks.remainder().is_empty() {
      return Err(invalid("incomplete f32 bytes"));
    }
    for chunk in chunks {
      let encoded = chunk
        .try_into()
        .map_err(|error| invalid(&format!("incomplete f32 bytes: {error}")))?;
      elements.push(f32::from_le_bytes(encoded));
    }
    Self::new(&elements)
  }
}

impl<const N: usize> From<Vector<N>> for Value {
  fn from(vector: Vector<N>) -> Self {
    Self::Vector(vector.0)
  }
}

#[cfg(feature = "lancedb")]
impl<const N: usize> crate::row::FromLanceValue for Vector<N> {
  fn from_lance_value(value: &Value) -> Option<Self> {
    if let Value::Vector(value) = value {
      Self::new(value.as_slice()).ok()
    } else {
      None
    }
  }
}
