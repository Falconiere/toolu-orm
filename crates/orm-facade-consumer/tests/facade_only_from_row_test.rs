//! `#[derive(FromRow)]` expands with `toolu-orm` as the only dependency.
//!
//! # Public API
//!
//! Tests: the derived `FromRow` impl reports its required columns.
//!
//! The derive names the active driver's row type and reaches
//! `impl_derived_from_row!` at `toolu-orm-core`'s root, neither of which this
//! package depends on — both resolve through the facade
//! (`::toolu_orm::core::…`), which is why the file compiling at all is the
//! assertion that matters. It compiles on every lane, so the default lane's
//! single-driver shape is covered here too.

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
