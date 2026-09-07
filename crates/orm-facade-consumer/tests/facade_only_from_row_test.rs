//! `#[derive(FromRow)]` expands with `toolu-orm` as the only dependency.
//!
//! # Public API
//!
//! Tests: the derived `FromRow` impl reports its required columns.
//!
//! The derive emits the postgres+libsql trait shape and names
//! `tokio_postgres::Row` / `libsql::Row`, none of which this package depends
//! on — they resolve through `toolu-orm-core`'s re-exports, which is why the
//! file compiling at all is the assertion that matters. Postgres lane only.

use toolu_orm::core::row::FromRow;
use toolu_orm::FromRow;

#[derive(FromRow)]
pub struct FacadeOnlyRow {
  pub id: String,
  pub email: String,
  pub age: i64,
}

#[test]
fn from_row_derive_reports_required_columns() {
  assert_eq!(FacadeOnlyRow::REQUIRED_COLUMNS, &["id", "email", "age"]);
}
