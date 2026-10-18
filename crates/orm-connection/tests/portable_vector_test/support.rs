use toolu_orm_core::{value::Value, vector::Vector};
pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const ELEMENTS: [f32; 3] = [1.25, -2.5, 0.0];
/// Builds a checked three-dimensional parameter for the native fixtures.
///
/// # Errors
/// Returns an error for a dimension mismatch or any non-finite element.
pub fn value(elements: &[f32]) -> Result<Value, toolu_orm_core::error::DbCoreError> {
  Ok(Vector::<3>::new(elements)?.into())
}
pub fn invalid_inputs() -> Vec<Vec<f32>> {
  vec![
    vec![],
    vec![1.0, 2.0],
    vec![1.0; 4],
    vec![1.0, f32::NAN, 3.0],
    vec![1.0, f32::INFINITY, 3.0],
    vec![1.0, f32::NEG_INFINITY, 3.0],
  ]
}

pub const FINITE_BOUNDARIES: [[f32; 3]; 2] =
  [[0.0; 3], [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1)]];
