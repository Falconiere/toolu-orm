//! Invariant-preserving, dimension-erased storage for a checked vector.
use crate::error::DbCoreError;

/// Finite f32 elements with a positive, validated dimension.
///
/// Fields are private so a [`crate::value::Value::Vector`] cannot carry an
/// unchecked vector. Prefer [`super::Vector`] for a statically declared size.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorValue {
  elements: Vec<f32>,
}

impl VectorValue {
  /// Validate before constructing a bindable value.
  ///
  /// # Errors
  /// Returns `VectorDimension` for a length mismatch, or `InvalidParameter`
  /// for dimension zero or a nonfinite element.
  pub fn new(elements: &[f32], dim: u32) -> Result<Self, DbCoreError> {
    if elements.len() != dim as usize {
      return Err(DbCoreError::VectorDimension {
        expected: dim,
        actual: elements.len(),
      });
    }
    if dim == 0 {
      return Err(invalid("dimension must be positive"));
    }
    if let Some(index) = elements.iter().position(|element| !element.is_finite()) {
      return Err(invalid(&format!("element {index} must be finite")));
    }
    Ok(Self {
      elements: elements.to_vec(),
    })
  }

  /// Validated elements, without mutable access.
  #[must_use]
  pub fn as_slice(&self) -> &[f32] {
    &self.elements
  }

  /// Little-endian sqlite-vec float bytes. Not the Lance representation.
  #[must_use]
  pub fn sqlite_bytes(&self) -> Vec<u8> {
    self
      .elements
      .iter()
      .flat_map(|element| element.to_le_bytes())
      .collect()
  }

  /// Bracketed round-trippable decimal f32 text for native vector input.
  ///
  /// Lance binds this as text into a declared FLOAT[N] column because the
  /// pinned DuckDB Rust driver cannot bind container parameters directly.
  #[must_use]
  pub fn array_text(&self) -> String {
    format!(
      "[{}]",
      self
        .elements
        .iter()
        .map(f32::to_string)
        .collect::<Vec<_>>()
        .join(",")
    )
  }
}

pub(super) fn invalid(reason: &str) -> DbCoreError {
  DbCoreError::InvalidParameter {
    kind: "vector",
    reason: reason.into(),
  }
}
