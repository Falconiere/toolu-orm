//! The facade's re-exports and its prelude.
//!
//! # Public API
//!
//! Tests: `#[table]` expands through the facade, the generated builders reach
//! `toolu-orm-query`, and `toolu_orm::prelude` still puts `toolu_orm_core` /
//! `toolu_orm_query` in scope for code that names them.
//!
//! This package depends on the four library crates directly, so it cannot
//! prove the single-dependency case — `crates/orm-facade-consumer` does that.
//! What it proves here is that the re-exports and the prelude keep working.

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
    def
      .columns
      .iter()
      .map(|c| c.name.as_str())
      .collect::<Vec<_>>(),
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

#[test]
fn prelude_still_exports_the_expansion_crate_names() {
  // Named through the glob above, not through `toolu_orm::core` / `::query`.
  let def: toolu_orm_core::table::TableDef = FacadeUser::table_def();
  let builder = toolu_orm_query::select::SelectBuilder::new(&def.name);

  assert_eq!(builder.table_name(), "facade_users");
}
