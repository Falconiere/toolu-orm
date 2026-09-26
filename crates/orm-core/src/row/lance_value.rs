//! Strict scalar conversions for owned Lance result values.

use crate::value::Value;

/// Convert one portable Lance scalar to an owned Rust field.
///
/// Implementations must return `None` for an incompatible value. Column and
/// expected-type diagnostics are added by [`super::LanceRow::get_typed`].
/// Built-in conversions never coerce between scalar kinds.
pub trait FromLanceValue: Sized {
  /// Return the decoded field, or `None` when the value is incompatible.
  fn from_lance_value(value: &Value) -> Option<Self>;
}

macro_rules! scalar {
  ($ty:ty, $variant:ident) => {
    impl FromLanceValue for $ty {
      fn from_lance_value(value: &Value) -> Option<Self> {
        if let Value::$variant(value) = value {
          Some(value.clone())
        } else {
          None
        }
      }
    }
  };
}

scalar!(i64, Integer);
scalar!(f64, Real);
scalar!(bool, Boolean);
scalar!(String, Text);
scalar!(Vec<u8>, Blob);

impl<T: FromLanceValue> FromLanceValue for Option<T> {
  fn from_lance_value(value: &Value) -> Option<Self> {
    if matches!(value, Value::Null) {
      Some(None)
    } else {
      T::from_lance_value(value).map(Some)
    }
  }
}
