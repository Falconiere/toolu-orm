//! Parameter conversion for the rusqlite backend.

use toolu_orm_core::value::Value;

/// Borrow the parameters as rusqlite's dynamic parameter type.
pub(super) fn to_sql_params(params: &[Value]) -> Vec<&dyn rusqlite::types::ToSql> {
  params
    .iter()
    .map(|v| v as &dyn rusqlite::types::ToSql)
    .collect()
}
