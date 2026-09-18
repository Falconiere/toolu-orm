//! Shared setup for the Postgres scalar-expression suite.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;

use crate::seed::{CREATED_AT, LAST_ACCESSED, MEMORY_COLUMNS};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const ESCAPE: char = '\\';

/// Every column of `memories`, in `Memory` field order.
pub fn all_memories() -> SelectBuilder {
  SelectBuilder::new("memories").columns_raw(&MEMORY_COLUMNS)
}

/// `coalesce("last_accessed", "created_at")`.
///
/// # Errors
///
/// [`DbCoreError::InvalidScalarFunction`] — unreachable for this literal name.
pub fn effective_time() -> Result<Scalar, DbCoreError> {
  Scalar::func(
    "coalesce",
    vec![Scalar::col(&LAST_ACCESSED), Scalar::col(&CREATED_AT)],
  )
}

/// `id` plus one computed column, decoded positionally.
#[derive(FromRow, Debug, PartialEq)]
pub struct Effective {
  pub id: String,
  pub effective: String,
}

/// `id` plus two computed columns.
#[derive(FromRow, Debug, PartialEq)]
pub struct Labelled {
  pub id: String,
  pub heat: String,
  pub tagged: String,
}

/// One computed column that may be NULL.
#[derive(FromRow, Debug, PartialEq)]
pub struct MaybeLabel {
  pub label: Option<String>,
}
