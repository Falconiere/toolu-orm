//! The facade is usable with `toolu-orm` as the only dependency.
//!
//! # Public API
//!
//! Tests: `toolu_orm::prelude` puts the macro-expansion crate names in scope,
//! `#[table]` produces a schema, and the generated builders reach
//! `toolu-orm-query` through the facade.
//!
//! This file deliberately names no `toolu_orm_*` crate directly: only
//! `toolu-orm` is in Cargo.toml, so anything it reaches has to come through
//! the facade's re-exports.

use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::table::TableSchema;
use toolu_orm::prelude::*;

#[table(name = "facade_users")]
pub struct FacadeUser {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  pub age: Integer,
}

#[test]
fn table_macro_expands_through_the_facade() {
  let def = FacadeUser::table_def();

  assert_eq!(def.name, "facade_users");
  assert_eq!(
    def.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
    vec!["id", "email", "age"]
  );
  assert_eq!(facade_users::TABLE, "facade_users");
  assert_eq!(facade_users::ALL_COLUMNS, &["id", "email", "age"]);
}

#[test]
fn generated_builders_reach_the_query_crate() {
  let (sql, params) = FacadeUser::select().columns_raw(&["id", "email"]).to_sql();

  assert_eq!(sql, r#"SELECT "id", "email" FROM "facade_users""#);
  assert!(params.is_empty());
}
