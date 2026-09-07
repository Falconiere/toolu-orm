//! Rusqlite `SelectBuilder` fetch methods.

use super::shared::impl_sync_fetch;

struct ScalarI64 {
  value: i64,
}

impl toolu_orm_core::row::FromRow for ScalarI64 {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["scalar"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, toolu_orm_core::error::DbCoreError> {
    Ok(Self {
      value: row.get(0).map_err(|e| {
        toolu_orm_core::error::DbCoreError::RowMapping(format!("scalar col: {e}"))
      })?,
    })
  }
}

impl_sync_fetch!(ScalarI64);
