//! Tests for companion columns module and builder factory methods.
//!
//! # Public API
//!
//! Tests: column module generation, Column expression ops, builder factory
//! methods (select, insert, update, delete, select_for).

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_macros::table;

// --- Companion columns module ---

#[table(name = "col_test")]
pub struct ColTestRow {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub name: Text,
  #[column(not_null, default = "unixepoch()")]
  pub created_at: Integer,
}

#[test]
fn table_generates_column_module_with_table_and_columns() {
  assert_eq!(col_test::TABLE, "col_test");
  assert_eq!(col_test::ALL_COLUMNS, &["id", "name", "created_at"]);
}

#[table(name = "expr_test")]
pub struct ExprTestRow {
  #[column(primary_key)]
  pub id: Text,
  pub score: Integer,
}

#[test]
fn column_constants_produce_expressions() {
  use toolu_orm_core::query_column::CommonOps;

  let expr = expr_test::id.eq("abc");
  let (sql, _) = expr.to_sql_fragment(1);
  assert!(sql.contains("\"expr_test\".\"id\""));
}

// --- Builder factory methods ---

#[table(name = "builder_test")]
pub struct BuilderTestRow {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub name: Text,
}

#[derive(toolu_orm_macros::FromRow)]
pub struct BuilderTestFromRow {
  pub id: String,
  pub name: String,
}

#[test]
fn table_generates_builder_factory_methods() {
  let _select = BuilderTestRow::select();
  let _select_for = BuilderTestRow::select_for::<BuilderTestFromRow>();
  let _insert = BuilderTestRow::insert();
  let _update = BuilderTestRow::update();
  let _delete = BuilderTestRow::delete();

  let (sql, _) = BuilderTestRow::select()
    .columns_raw(&["id", "name"])
    .to_sql();
  assert_eq!(sql, r#"SELECT "id", "name" FROM "builder_test""#);

  let (sql, _) = BuilderTestRow::select_for::<BuilderTestFromRow>().to_sql();
  assert_eq!(sql, r#"SELECT "id", "name" FROM "builder_test""#);
}
