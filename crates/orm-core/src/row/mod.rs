//! Row decoding (`FromRow`) and Postgres helpers.

#[cfg(feature = "postgres")]
mod decode_postgres;
#[cfg(feature = "postgres")]
mod pg_count_scalar;
mod traits;

#[cfg(feature = "postgres")]
pub use decode_postgres::from_postgres_row;
#[cfg(feature = "postgres")]
pub use pg_count_scalar::PgCountScalar;
pub use traits::FromRow;

/// Generates feature-gated `FromRow` impls for a type across backend combinations.
///
/// Supports single-backend (one `from_row`), dual-backend, and triple-backend shapes.
#[macro_export]
macro_rules! impl_from_row_for {
  (single $cfg:meta, $ty:ty, $cols:expr, $method:ident, $row_ty:ty, $body:expr) => {
    #[$cfg]
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn $method(row: &$row_ty) -> Result<Self, $crate::error::DbCoreError> {
        $body(row)
      }
    }
  };
  (dual $cfg:meta, $ty:ty, $cols:expr,
    [$m1:ident $r1:ty => $b1:expr],
    [$m2:ident $r2:ty => $b2:expr]
  ) => {
    #[$cfg]
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn $m1(row: &$r1) -> Result<Self, $crate::error::DbCoreError> {
        $b1(row)
      }
      fn $m2(row: &$r2) -> Result<Self, $crate::error::DbCoreError> {
        $b2(row)
      }
    }
  };
  (triple $cfg:meta, $ty:ty, $cols:expr,
    [$m1:ident $r1:ty => $b1:expr],
    [$m2:ident $r2:ty => $b2:expr],
    [$m3:ident $r3:ty => $b3:expr]
  ) => {
    #[$cfg]
    impl $crate::row::FromRow for $ty {
      const REQUIRED_COLUMNS: &'static [&'static str] = $cols;
      fn $m1(row: &$r1) -> Result<Self, $crate::error::DbCoreError> {
        $b1(row)
      }
      fn $m2(row: &$r2) -> Result<Self, $crate::error::DbCoreError> {
        $b2(row)
      }
      fn $m3(row: &$r3) -> Result<Self, $crate::error::DbCoreError> {
        $b3(row)
      }
    }
  };
}
