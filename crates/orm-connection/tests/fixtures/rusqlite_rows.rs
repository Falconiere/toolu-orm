//! Row decoding shared by the rusqlite suites.
//!
//! Wired into `rusqlite_impl_test.rs` and `rusqlite_blocking_test.rs` with
//! `#[path]`; every item here is used by both binaries (no dead code under
//! `-D warnings`).
//!
//! Single-backend shape: with rusqlite as orm-core's sole driver feature,
//! `FromRow` exposes `from_row(&rusqlite::Row)`.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;

/// Any one-column integer result -- a `count`, a `PRAGMA` value. It reads
/// column 0 positionally, so it names no required column.
pub struct ScalarRow {
  pub value: i64,
}

impl FromRow for ScalarRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &[];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let value: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { value })
  }
}

/// A one-column text result, for asserting on what was actually stored.
pub struct LabelRow {
  pub label: String,
}

impl FromRow for LabelRow {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["label"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let label: String = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { label })
  }
}
