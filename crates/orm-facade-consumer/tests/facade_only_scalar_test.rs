//! Scalar expressions and `LIKE … ESCAPE` compose with `toolu-orm` as the
//! only dependency.
//!
//! Like its sibling binaries this package depends on `toolu-orm` alone, so
//! `toolu_orm_core` is absent from the extern prelude and every path here is
//! written through the facade.

use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::error::DbCoreError;
use toolu_orm::core::expr::{like_pattern_literal, Scalar};
use toolu_orm::core::query_column::TextOps;
use toolu_orm::table;

#[table(name = "facade_scalar_rows")]
pub struct FacadeScalarRow {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  pub age: Integer,
}

/// Rendered for an explicit dialect: `to_sql()` follows `Dialect::CURRENT`,
/// which is Postgres (`$N`) in the postgres lane and SQLite (`?N`) elsewhere,
/// and this asserts the placeholder text.
#[test]
fn scalar_expressions_compose_through_the_facade() -> Result<(), DbCoreError> {
  let bumped = Scalar::col(&facade_scalar_rows::age) + Scalar::bind(1);
  let (sql, params) = FacadeScalarRow::select()
    .columns_raw(&["id"])
    .column_scalar(bumped, "next_age")
    .filter(
      facade_scalar_rows::email
        .like_escape(format!("%{}%", like_pattern_literal("a_b", '\\')), '\\'),
    )
    .order_by(Scalar::func("lower", vec![Scalar::col(&facade_scalar_rows::email)])?.asc())
    .to_sql_for(Dialect::Sqlite);

  assert_eq!(
    sql,
    r#"SELECT "id", ("facade_scalar_rows"."age" + ?1) AS "next_age" FROM "facade_scalar_rows" "#
      .to_owned()
      + r#"WHERE "facade_scalar_rows"."email" LIKE ?2 ESCAPE ?3 "#
      + r#"ORDER BY lower("facade_scalar_rows"."email") ASC"#
  );
  assert_eq!(params.len(), 3);
  Ok(())
}
