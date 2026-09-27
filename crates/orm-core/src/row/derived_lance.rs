//! Core-selected Lance method for derived row implementations.

/// Internal derive helper: emit the Lance method using core's unified features.
///
/// This macro expands inside an impl. Its definition, rather than the generated
/// method, is cfg-gated so the consumer does not need matching feature names.
#[doc(hidden)]
#[cfg(feature = "lancedb")]
#[macro_export]
macro_rules! impl_derived_lance_row {
  (|$row:ident| $body:block) => {
    fn from_lance_row($row: &$crate::row::LanceRow)
      -> Result<Self, $crate::error::DbCoreError> $body
  };
}

/// Internal derive helper: discard Lance tokens before name resolution.
#[doc(hidden)]
#[cfg(not(feature = "lancedb"))]
#[macro_export]
macro_rules! impl_derived_lance_row {
  (|$row:ident| $body:block) => {};
}
